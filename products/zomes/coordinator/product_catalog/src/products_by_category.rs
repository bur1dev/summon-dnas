use hdk::prelude::*;
use products_integrity::*;
use crate::utils::concurrent_get_records;

// Define the constant here or import if defined elsewhere
// const PRODUCTS_PER_GROUP: usize = 1000;

// Updated to return product groups instead of individual products
#[derive(Serialize, Deserialize, Debug)]
pub struct CategorizedProducts {
    pub category: String,
    pub subcategory: Option<String>,
    pub product_type: Option<String>,
    pub product_groups: Vec<Record>,          // Now contains ProductGroup records
    pub total_groups: usize,                  // Total number of groups for this category path
    pub total_products: usize,                // Estimated total number of products across *all* groups for this category path
    pub has_more: bool,                       // Indicates if there are more groups beyond the current page
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetProductsParams {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub subcategory: Option<String>,
    #[serde(default)]
    pub product_type: Option<String>,
    #[serde(default)] // Represents the group offset
    pub offset: usize,
    #[serde(default = "default_limit")] // Represents the group limit
    pub limit: usize,
}

fn default_limit() -> usize {
    // Default limit for number of *groups* to fetch per request
    // Corresponds to PRODUCTS_PER_GROUP * default_limit() products roughly
    // Let's set a reasonable default, e.g., 5 groups (100 products)
    5
}

// Modified to work with product groups and return correct total_products
#[hdk_extern]
pub fn get_products_by_category(params: GetProductsParams) -> ExternResult<CategorizedProducts> {
    // Determine the path based on category/subcategory/product_type
    let base_path = match (&params.subcategory, &params.product_type) {
        (Some(subcategory), Some(product_type)) => format!(
            "categories/{}/subcategories/{}/types/{}", 
            params.category, subcategory, product_type
        ),
        (Some(subcategory), None) => format!(
            "categories/{}/subcategories/{}", 
            params.category, subcategory
        ),
        (None, None) => format!("categories/{}", params.category),
        (None, Some(_)) => {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Cannot have product type without subcategory".into()
            )))
        }
    };

    let chunk_path = Path::try_from(base_path.clone())?;
    let path_hash = chunk_path.path_entry_hash()?;

    let all_links = match get_links(
        GetLinksInputBuilder::try_new(path_hash.clone(), LinkTypes::ProductTypeToGroup)?.build(),
    ) {
        Ok(links) => links,
        Err(e) => {
            return Err(e);
        }
    };
    
    let total_groups = all_links.len();
    
    // Calculate total products count from link tags
    let total_products_from_links: usize = all_links.iter()
        .map(|link| {
            if link.tag.0.len() >= 4 {
                let count_bytes: [u8; 4] = link.tag.0[..4].try_into().unwrap_or([0, 0, 0, 0]);
                u32::from_le_bytes(count_bytes) as usize
            } else { 0 }
        })
        .sum();
    
    // Apply pagination directly (no sorting needed)
    let paginated_links = all_links
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .collect::<Vec<_>>();

    // Extract action hashes from the paginated links
    let target_hashes: Vec<_> = paginated_links
        .into_iter()
        .filter_map(|link| link.target.into_action_hash())
        .collect();

    // Get the records for the paginated product groups
    let product_groups_records = concurrent_get_records(target_hashes)?;

    // Calculate total products count
    let mut total_products_count = 0;

    for record in product_groups_records.iter() {
        match record.entry().to_app_option::<ProductGroup>() {
            Ok(Some(group)) => {
                total_products_count += group.products.len();
            },
            Ok(None) => {},
            Err(_e) => {}
        }
    }

    // Determine if there are more groups beyond the current page
    let has_more = (params.offset + params.limit) < total_groups;

    Ok(CategorizedProducts {
        category: params.category,
        subcategory: params.subcategory,
        product_type: params.product_type,
        product_groups: product_groups_records,
        total_groups,
        total_products: total_products_from_links, // Use link tag totals instead of fetched groups
        has_more,
    })
}

#[hdk_extern]
pub fn get_all_category_products(category: String) -> ExternResult<CategorizedProducts> {
    let path_str = format!("categories/{}", category);
    
    let chunk_path = match Path::try_from(path_str.clone()) {
        Ok(path) => path,
        Err(e) => {
            return Err(e.into());
        }
    };
    
    let path_hash = match chunk_path.path_entry_hash() {
        Ok(hash) => hash,
        Err(e) => {
            return Err(e);
        }
    };
    
    // Get links to product groups at the category level
    let links = match get_links(
        GetLinksInputBuilder::try_new(path_hash, LinkTypes::ProductTypeToGroup)?.build()
    ) {
        Ok(links) => links,
        Err(e) => {
            return Err(e);
        }
    };

    let total_groups = links.len();

    // Log all links found with their chunk IDs
    for (_i, link) in links.iter().enumerate() {
        let _chunk_id = if link.tag.0.len() >= 4 {
            u32::from_le_bytes(link.tag.0[..4].try_into().unwrap_or([0, 0, 0, 0]))
        } else {
            0
        };
    }

    // Extract action hashes from links
    let all_hashes: Vec<_> = links
        .into_iter()
        .filter_map(|link| {
            let hash_opt = link.target.clone().into_action_hash();
            if hash_opt.is_none() {
            }
            hash_opt
        })
        .collect();
    
    // Get all product group records
    // Get all product group records
let product_groups_records = match concurrent_get_records(all_hashes.clone()) {
    Ok(records) => {
        if records.len() < all_hashes.len() {
            // Log the missing hashes
            let returned_hashes: Vec<ActionHash> = records.iter()
                .map(|r| r.action_address().clone())
                .collect();
            for (_i, hash) in all_hashes.iter().enumerate() {
                if !returned_hashes.contains(hash) {
                }
            }
        }
        records
    },
    Err(e) => {
        return Err(e);
    }
};
    
    let mut _deserialize_success = 0;
    let mut _deserialize_empty = 0;
    let mut _deserialize_error = 0;
    let mut _total_products_count = 0;
    
    for (_i, record) in product_groups_records.iter().enumerate() {
        match record.entry().to_app_option::<ProductGroup>() {
            Ok(Some(group)) => {
                let product_count = group.products.len();
                _total_products_count += product_count;
                _deserialize_success += 1;
                
            },
            Ok(None) => {
                _deserialize_empty += 1;
            },
            Err(_e) => {
                _deserialize_error += 1;
            }
        }
    }

    // Calculate total number of products across ALL fetched groups
    let actual_total_products = product_groups_records.iter()
        .filter_map(|record| {
            record.entry().to_app_option::<ProductGroup>().ok()? // Get ProductGroup entry
        })
        .map(|group| group.products.len()) // Get count of products in each group
        .sum(); // Sum counts

    Ok(CategorizedProducts {
        category,
        subcategory: None,
        product_type: None,
        product_groups: product_groups_records.clone(),
        total_groups,
        total_products: actual_total_products,
        has_more: false,
    })
}

// New function to get paginated products from a group
#[derive(Serialize, Deserialize, Debug)]
pub struct GroupProductsParams {
    pub group_hash: ActionHash,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaginatedProducts {
    pub products: Vec<Product>,
    pub total: usize,
    pub has_more: bool,
}

#[hdk_extern]
pub fn get_paginated_products_from_group(params: GroupProductsParams) -> ExternResult<PaginatedProducts> {
    // Get the product group record
     let group_record = match get(params.group_hash.clone(), GetOptions::default())? {
         Some(record) => record,
         None => {
             return Err(wasm_error!(WasmErrorInner::Guest("Group not found".into())));
         }
    };

    // Extract the ProductGroup from the record
    let product_group = match ProductGroup::try_from(group_record) {
         Ok(group) => group,
         Err(e) => {
             return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to deserialize ProductGroup: {:?}", e))));
         }
    };

    let total = product_group.products.len();

    // Apply pagination
    let products: Vec<Product> = product_group.products
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .collect();

    let has_more = (params.offset + params.limit) < total;

    Ok(PaginatedProducts {
        products,
        total,
        has_more,
    })
}
