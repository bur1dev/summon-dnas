use hdk::prelude::*;
use products_integrity::*;
use std::collections::HashMap;
use crate::utils::concurrent_get_records;
use crate::products_by_category::GetProductsParams;
// Constants remain the same
pub const PRODUCTS_PER_GROUP: usize = 1000; // Maximum products per group

// Get appropriate paths for a product or product group
pub fn get_paths(input: &CreateProductInput) -> ExternResult<Vec<Path>> {
    let mut paths = Vec::new();
    let mut path_strings = Vec::new(); // For logging

    // Main category path
    let main_path_str = format!("categories/{}", input.main_category);
    paths.push(Path::try_from(main_path_str.clone())?);
    path_strings.push(main_path_str);


    if let Some(subcategory) = &input.subcategory {
        // Subcategory path
        let sub_path_str = format!(
            "categories/{}/subcategories/{}", 
            input.main_category, subcategory
        );
        paths.push(Path::try_from(sub_path_str.clone())?);
        path_strings.push(sub_path_str);


        if let Some(product_type) = &input.product_type {
            // Product type path
            let type_path_str = format!(
                "categories/{}/subcategories/{}/types/{}", 
                input.main_category, subcategory, product_type
            );
            paths.push(Path::try_from(type_path_str.clone())?);
            path_strings.push(type_path_str);
        }
    }

    // Handle additional categorization paths
for (_i, additional) in input.additional_categorizations.iter().enumerate() {
    // Main category for additional categorization
    let additional_main_path_str = format!("categories/{}", additional.main_category);
    paths.push(Path::try_from(additional_main_path_str.clone())?);
    path_strings.push(additional_main_path_str);

    if let Some(subcategory) = &additional.subcategory {
        let additional_sub_path_str = format!(
            "categories/{}/subcategories/{}", 
            additional.main_category, subcategory
        );
        paths.push(Path::try_from(additional_sub_path_str.clone())?);
        path_strings.push(additional_sub_path_str);

        if let Some(product_type) = &additional.product_type {
            let additional_type_path_str = format!(
                "categories/{}/subcategories/{}/types/{}", 
                additional.main_category, subcategory, product_type
            );
            paths.push(Path::try_from(additional_type_path_str.clone())?);
            path_strings.push(additional_type_path_str);
        }
    }
}

    Ok(paths)
}

fn create_links_for_group(group_hash: &ActionHash, paths: Vec<Path>, product_count: usize) -> ExternResult<()> {
    // Convert product count to bytes for LinkTag
    let count_bytes = (product_count as u32).to_le_bytes();
    let link_tag = LinkTag::new(count_bytes.to_vec());

    for path in paths.iter() {
        match path.path_entry_hash() {
            Ok(path_hash) => {
                create_link(
                    path_hash.clone(),
                    group_hash.clone(),
                    LinkTypes::ProductTypeToGroup,
                    link_tag.clone(),
                )?;
            },
            Err(_e) => {}
        }
    }
    Ok(())
}


// New function to create product groups
#[hdk_extern]
pub fn create_product_group(input: CreateProductGroupInput) -> ExternResult<ActionHash> {
    let product_group = ProductGroup {
        category: input.category.clone(),
        subcategory: input.subcategory.clone(),
        product_type: input.product_type.clone(),
        products: input.products,
        additional_categorizations: input.additional_categorizations.clone(),
    };

    let group_hash = create_entry(&EntryTypes::ProductGroup(product_group.clone()))?;

    let first_product = product_group.products.first()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Product group is empty".into())))?;

    let mock_input = CreateProductInput {
        product: first_product.clone(),
        main_category: input.category,
        subcategory: input.subcategory,
        product_type: input.product_type,
        additional_categorizations: input.additional_categorizations.clone(),
    };

    let paths = get_paths(&mock_input)?;
    let product_count = product_group.products.len();
    create_links_for_group(&group_hash, paths, product_count)?;

    Ok(group_hash)
}

#[hdk_extern]
pub fn create_product_batch(products: Vec<CreateProductInput>) -> ExternResult<Vec<Record>> {
    if products.is_empty() {
        return Ok(Vec::new());
    }

    // Group products by PRIMARY category
    let mut grouped_products: HashMap<String, Vec<CreateProductInput>> = HashMap::new();
    for mut product_input in products {
        // Normalize input fields
        if product_input.subcategory == Some("".to_string()) { product_input.subcategory = None; }
        if product_input.product_type == Some("".to_string()) { product_input.product_type = None; }
        if product_input.product.subcategory == Some("".to_string()) { product_input.product.subcategory = None; }
        if product_input.product.product_type == Some("".to_string()) { product_input.product.product_type = None; }

        // Ensure product fields match input fields
        if product_input.product.category != product_input.main_category {
            product_input.product.category = product_input.main_category.clone();
        }
        if product_input.product.subcategory != product_input.subcategory {
            product_input.product.subcategory = product_input.subcategory.clone();
        }
        if product_input.product.product_type != product_input.product_type {
            product_input.product.product_type = product_input.product_type.clone();
        }

        let key = format!("{}||{}||{}", 
            product_input.main_category.clone(),
            product_input.subcategory.clone().unwrap_or_default(),
            product_input.product_type.clone().unwrap_or_default()
        );

        grouped_products.entry(key).or_insert_with(Vec::new).push(product_input);
    }

    let mut all_group_hashes = Vec::new();

    // Process each PRIMARY category group
    for (key, group_products_inputs) in grouped_products {
        let parts: Vec<&str> = key.split("||").collect();
        let group_primary_category = parts[0].to_string();
        let group_primary_subcategory = if parts[1].is_empty() { None } else { Some(parts[1].to_string()) };
        let group_primary_product_type = if parts[2].is_empty() { None } else { Some(parts[2].to_string()) };

        // Create products for group
        let products_for_group: Vec<_> = group_products_inputs.iter().map(|input| {
            let mut product = input.product.clone();
            product.category = group_primary_category.clone();
            product.subcategory = group_primary_subcategory.clone();
            product.product_type = group_primary_product_type.clone();
            product
        }).collect();

        let additional_categorizations = group_products_inputs.first()
            .map(|input| input.additional_categorizations.clone())
            .unwrap_or_default();

        let group_input = CreateProductGroupInput {
            category: group_primary_category.clone(),
            subcategory: group_primary_subcategory.clone(),
            product_type: group_primary_product_type.clone(),
            products: products_for_group,
            additional_categorizations,
        };

        match create_product_group(group_input) {
            Ok(hash) => {
                all_group_hashes.push(hash);
            },
            Err(_e) => {
                // Continue with other groups even if one fails
            }
        }
    }

    // Retrieve records for all successfully created groups
    concurrent_get_records(all_group_hashes)
}

// Function to get an individual product group by hash
#[hdk_extern]
pub fn get_product_group(hash: ActionHash) -> ExternResult<Option<Record>> {
    let result = get(hash, GetOptions::default());
    result
}


#[hdk_extern]
pub fn get_all_group_counts_for_path(params: GetProductsParams) -> ExternResult<Vec<usize>> {
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

    let chunk_path = match Path::try_from(base_path.clone()) {
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

    let all_links = match get_links(
        GetLinksInputBuilder::try_new(path_hash, LinkTypes::ProductTypeToGroup)?
            .build(),
    ) {
        Ok(links) => links,
        Err(e) => {
            return Err(e);
        }
    };

    // Read counts directly from link tags instead of fetching groups
    let mut counts = Vec::new();
    
    for link in all_links.iter() {
        // Extract count from link tag
        if link.tag.0.len() >= 4 {
            let count_bytes: [u8; 4] = link.tag.0[..4].try_into().unwrap_or([0, 0, 0, 0]);
            let count = u32::from_le_bytes(count_bytes) as usize;
            counts.push(count);
        } else {
            // Fallback to 0 if tag is malformed
            counts.push(0);
        }
    }
    
    Ok(counts)
}

#[hdk_extern]
pub fn get_product_groups_by_path(params: GetProductGroupsParams) -> ExternResult<Vec<Record>> {
    debug!("🔍 get_product_groups_by_path called with: category={}, subcategory={:?}, product_type={:?}",
        params.category, params.subcategory, params.product_type);
    
    // Construct the path based on category/subcategory/product_type
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

    debug!("🛣️ Using path: {}", base_path);
    
    let path = Path::try_from(base_path.clone())?;
    let path_hash = path.path_entry_hash()?;

    debug!("🔑 Path hash: {}", path_hash);

    // Get links to product groups at this path
    let links = get_links(
        GetLinksInputBuilder::try_new(path_hash, LinkTypes::ProductTypeToGroup)?.build()
    )?;
    
    debug!("🔗 Found {} links at path", links.len());
    
    if links.is_empty() {
        return Ok(Vec::new());
    }

    // Extract action hashes from links
    let target_hashes: Vec<_> = links
        .into_iter()
        .filter_map(|link| link.target.into_action_hash())
        .collect();
    
    debug!("🎯 Retrieving {} product group records", target_hashes.len());
    
    // Get all product group records
    let records = concurrent_get_records(target_hashes)?;
    debug!("✅ Retrieved {} product group records", records.len());
    
    Ok(records)
}

// Parameter struct for the get_product_groups_by_path function
#[derive(Serialize, Deserialize, Debug)]
pub struct GetProductGroupsParams {
    pub category: String,
    pub subcategory: Option<String>,
    pub product_type: Option<String>,
}

// New function to delete links to a product group
#[hdk_extern]
pub fn delete_links_to_product_group(group_hash: ActionHash) -> ExternResult<()> {
    debug!("🗑️ delete_links_to_product_group called for hash: {}", group_hash);

    // Get the product group to find its category info
    let group_record = get(group_hash.clone(), GetOptions::default())?.ok_or(
        wasm_error!(WasmErrorInner::Guest("Group not found".into()))
    )?;
    
    let product_group = ProductGroup::try_from(group_record).map_err(|e| 
        wasm_error!(WasmErrorInner::Guest(format!("Failed to deserialize ProductGroup: {:?}", e)))
    )?;
    
    debug!("📦 Found product group: category={}, subcategory={:?}, product_type={:?}",
        product_group.category, product_group.subcategory, product_group.product_type);
    
    // Generate all possible paths for this product group
    // Now using the additional_categorizations stored ON the ProductGroup entry itself
    let mock_input = CreateProductInput {
        product: product_group.products.first().cloned().ok_or(
            wasm_error!(WasmErrorInner::Guest("Product group is empty".into()))
        )?,
        main_category: product_group.category.clone(), // Use .clone() for owned String
        subcategory: product_group.subcategory.clone(), // Use .clone() for Option<String>
        product_type: product_group.product_type.clone(), // Use .clone() for Option<String>
        additional_categorizations: product_group.additional_categorizations.clone(), // <-- USE STORED ADDITIONAL CATEGORIZATIONS
    };

    let paths = get_paths(&mock_input)?;
    debug!("🛣️ Generated {} paths to check for links", paths.len());
    
    let mut deleted_links = 0;
    let mut errors = 0;
    
    // For each path, find and delete links to this group
    for path in paths {
        match path.path_entry_hash() {
            Ok(path_hash) => {
                let links = get_links(
                    GetLinksInputBuilder::try_new(path_hash.clone(), LinkTypes::ProductTypeToGroup)?.build()
                )?;
                
                for link in links {
                    if let Some(target_hash) = link.target.into_action_hash() {
                        if target_hash == group_hash {
                            match delete_link(link.create_link_hash) {
                                Ok(_) => {
                                    deleted_links += 1;
                                    debug!("✅ Deleted link from path to group");
                                },
                                Err(e) => {
                                    errors += 1;
                                    debug!("❌ Failed to delete link: {:?}", e);
                                }
                            }
                        }
                    }
                }
            },
            Err(e) => {
                errors += 1;
                debug!("❌ Failed to get path hash: {:?}", e);
            }
        }
    }
    
    debug!("🗑️ Deleted {} links with {} errors", deleted_links, errors);
    
    if deleted_links == 0 && errors > 0 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Failed to delete any links to product group. Encountered {} errors", errors)
        )));
    }
    
    Ok(())
}

// New function that combines get_product_groups_by_path and delete_links_to_product_group
#[hdk_extern]
pub fn update_product_group(input: UpdateProductGroupInput) -> ExternResult<ActionHash> {
    debug!("🔄 update_product_group called");
    
    // 1. Get existing group(s) at the path
    let existing_groups = get_product_groups_by_path(GetProductGroupsParams {
        category: input.old_category.clone(),
        subcategory: input.old_subcategory.clone(),
        product_type: input.old_product_type.clone(),
    })?;
    
    if existing_groups.is_empty() {
        debug!("⚠️ No existing groups found at the old path");
    }
    
    // 2. Create new product group
    let new_group_hash = create_product_group(input.new_group)?;
    debug!("✅ Created new product group: {}", new_group_hash);
    
    // 3. Delete links to old groups
    for record in existing_groups {
        match delete_links_to_product_group(record.action_address().clone()) {
            Ok(_) => debug!("✅ Deleted links to old group: {}", record.action_address()),
            Err(e) => debug!("⚠️ Failed to delete links to old group: {:?}", e),
        }
    }
    
    Ok(new_group_hash)
}

// Parameter struct for the update_product_group function
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateProductGroupInput {
    pub old_category: String,
    pub old_subcategory: Option<String>,
    pub old_product_type: Option<String>,
    pub new_group: CreateProductGroupInput,
}