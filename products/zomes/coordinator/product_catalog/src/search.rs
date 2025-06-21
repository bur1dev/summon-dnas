use hdk::prelude::*;
use products_integrity::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchResult {
    pub products: Vec<Record>,
    pub total: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProductReference {
    pub group_hash: ActionHash,
    pub index: usize,
}

// New function to handle product references (group_hash + index)
#[hdk_extern]
pub fn get_products_by_references(references: Vec<ProductReference>) -> ExternResult<SearchResult> {
    
    if references.is_empty() {
        return Ok(SearchResult {
            products: vec![],
            total: 0,
        });
    }
    
    // Group references by group_hash to minimize fetches
    let mut group_map: std::collections::HashMap<ActionHash, Vec<usize>> = std::collections::HashMap::new();
    for reference in references {
        group_map
            .entry(reference.group_hash)
            .or_insert_with(Vec::new)
            .push(reference.index);
    }
    
    // Fetch all required ProductGroups
    let group_hashes: Vec<ActionHash> = group_map.keys().cloned().collect();
    
    let all_group_records = match get_records_from_hashes(group_hashes) {
        Ok(groups) => groups,
        Err(e) => {
            return Err(e);
        }
    };
    
    // Extract requested products from groups
    let mut product_records = Vec::new();
    
    for record in all_group_records {
        let group_hash = record.action_address().clone().into_hash();
if let Some(_indices) = group_map.get(&group_hash) { // Prefixed the first, shadowed `indices`
            if let Some(indices) = group_map.get(&group_hash) {
                // Extract ProductGroup from record
                if let Ok(Some(group)) = record.entry().to_app_option::<ProductGroup>() {
                    for &index in indices {
                        if index < group.products.len() {
                            // Create a virtual record for the product (containing the group record with group hash)
                            // This maintains compatibility with frontend expecting records
                            product_records.push(record.clone());
                        }
                    }
                } else {
                }
            }
        }
    }
    
    Ok(SearchResult {
        products: product_records.clone(),
        total: product_records.len(),
    })
}

fn get_records_from_hashes(hashes: Vec<ActionHash>) -> ExternResult<Vec<Record>> {
    // Process in batches of 100 to prevent timeouts
    const BATCH_SIZE: usize = 1000;
    let mut all_records = Vec::new();

    for (_batch_index, batch) in hashes.chunks(BATCH_SIZE).enumerate() {

        let input: Vec<_> = batch
            .iter()
            .map(|hash| GetInput::new(hash.clone().into(), GetOptions::default()))
            .collect();

        match HDK.with(|hdk| hdk.borrow().get(input)) {
            Ok(records) => {
                let valid_records: Vec<_> = records.into_iter().flatten().collect();
                all_records.extend(valid_records);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(all_records)
}

#[hdk_extern]
pub fn get_all_products_for_search_index() -> ExternResult<SearchResult> {
    
    // All categories that need to be indexed
    let categories = vec![
        "Produce", "Beverages", "Dairy & Eggs", "Snacks & Candy", "Meat & Seafood",
        "Wine", "Frozen", "Prepared Foods", "Liquor", "Floral", "Household", "Bakery",
        "Deli", "Canned Goods & Soups", "Beer", "Pets", "Breakfast", "Condiments & Sauces",
        "Personal Care", "Dry Goods & Pasta", "Oils, Vinegars, & Spices", "Health Care",
        "Baking Essentials", "Kitchen Supplies", "Hard Beverages", "Miscellaneous",
        "Party & Gift Supplies", "Office & Craft", "Baby"
    ];
    
    // Collect all unique ProductGroup hashes from all category paths
    let mut all_group_hashes = std::collections::HashSet::new();
    
    // Build all possible paths and collect links in parallel batches
    let mut all_paths = Vec::new();
    
    // Add main category paths
    for category in &categories {
        all_paths.push(format!("categories/{}", category));
    }
    
    // Process paths in batches to get all links
    const PATH_BATCH_SIZE: usize = 10;
    
    for path_batch in all_paths.chunks(PATH_BATCH_SIZE) {
        // Create GetLinks inputs for this batch
        let mut get_links_inputs = Vec::new();
        
        for path_str in path_batch {
            match Path::try_from(path_str.clone()) {
                Ok(path) => {
                    match path.path_entry_hash() {
                        Ok(path_hash) => {
                            match GetLinksInputBuilder::try_new(path_hash, LinkTypes::ProductTypeToGroup) {
                                Ok(builder) => {
                                    get_links_inputs.push(builder.build());
                                },
                                Err(_e) => {
                                }
                            }
                        },
                        Err(_e) => {
                        }
                    }
                },
                Err(_e) => {
                }
            }
        }
        
        // Get all links for this batch of paths
        match HDK.with(|hdk| hdk.borrow().get_links(get_links_inputs)) {
            Ok(link_results) => {
                for links in link_results {
                    for link in links {
                        if let Some(hash) = link.target.into_action_hash() {
                            all_group_hashes.insert(hash);
                        }
                    }
                }
            },
            Err(_e) => {
            }
        }
    }
    
    // Convert HashSet to Vec for concurrent_get_records
    let group_hashes: Vec<ActionHash> = all_group_hashes.into_iter().collect();
    
    // Fetch all ProductGroups using the efficient concurrent function
    match get_records_from_hashes(group_hashes) {
        Ok(records) => {
            
            // Count total products across all groups
            let mut total_products = 0;
            for record in &records {
                if let Ok(Some(group)) = record.entry().to_app_option::<ProductGroup>() {
                    total_products += group.products.len();
                }
            }
            
            Ok(SearchResult {
                products: records,
                total: total_products,
            })
        },
        Err(e) => {
            Err(e)
        }
    }
}
