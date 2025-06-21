use hdi::prelude::*;

#[derive(Clone, PartialEq)] // We only need Clone and PartialEq here if hdk_entry_helper provides the others
#[hdk_entry_helper]
pub struct Product {
    pub name: String,
    pub price: f32,
    pub promo_price: Option<f32>,
    pub size: String,
    pub stocks_status: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub product_type: Option<String>,
    pub image_url: Option<String>,
    pub sold_by: Option<String>,
    #[serde(rename = "productId")]
    pub product_id: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub brand: Option<String>,
    pub is_organic: Option<bool>,
}

// New ProductGroup struct that contains multiple products
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct ProductGroup {
    pub category: String,
    pub subcategory: Option<String>,
    pub product_type: Option<String>,
    pub products: Vec<Product>,
    pub additional_categorizations: Vec<DualCategorization>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProductInput {
    pub product: Product,
    pub main_category: String,
    pub subcategory: Option<String>,
    pub product_type: Option<String>,
    pub additional_categorizations: Vec<DualCategorization>,
}

// New input struct for creating product groups
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProductGroupInput {
    pub category: String,
    pub subcategory: Option<String>,
    pub product_type: Option<String>,
    pub products: Vec<Product>,
    pub additional_categorizations: Vec<DualCategorization>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DualCategorization {
    pub main_category: String,
    pub subcategory: Option<String>,
    pub product_type: Option<String>,
}

pub fn validate_create_product(
    _action: EntryCreationAction,
    product: Product,
) -> ExternResult<ValidateCallbackResult> {
    if product.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Product name cannot be empty".into(),
        ));
    }
    if product.price < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Price cannot be negative".into(),
        ));
    }

    // Validate sold_by if present
    if let Some(sold_by) = &product.sold_by {
        if sold_by != "WEIGHT" && sold_by != "UNIT" {
            return Ok(ValidateCallbackResult::Invalid(
                "sold_by must be either 'WEIGHT' or 'UNIT'".into(),
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

// New validation function for ProductGroup
pub fn validate_create_product_group(
    _action: EntryCreationAction,
    product_group: ProductGroup,
) -> ExternResult<ValidateCallbackResult> {
    if product_group.products.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Product group cannot be empty".into(),
        ));
    }

    // Add size limit check
    if product_group.products.len() > 1000 {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Product group exceeds maximum size of 100 products (found {})", 
                    product_group.products.len()).into(),
        ));
    }

    // Validate consistency - all products must match the group's category/subcategory/type
    for product in &product_group.products {
        if product.category != product_group.category {
            return Ok(ValidateCallbackResult::Invalid(
                "Product category does not match group category".into(),
            ));
        }
        
        // Compare subcategory while treating None and Some("") as equivalent
        let subcategory_matches = match (&product.subcategory, &product_group.subcategory) {
            (None, None) => true,
            (None, Some(s)) | (Some(s), None) => s.is_empty(),
            (Some(a), Some(b)) => a == b,
        };
        
        if !subcategory_matches {
            return Ok(ValidateCallbackResult::Invalid(
                "Product subcategory does not match group subcategory".into(),
            ));
        }
        
        // Compare product_type while treating None and Some("") as equivalent
        let product_type_matches = match (&product.product_type, &product_group.product_type) {
            (None, None) => true,
            (None, Some(s)) | (Some(s), None) => s.is_empty(),
            (Some(a), Some(b)) => a == b,
        };
        
        if !product_type_matches {
            return Ok(ValidateCallbackResult::Invalid(
                format!(
                    "Product type does not match group product type. Product: {:?}, Group: {:?}",
                    product.product_type, product_group.product_type
                ),
            ));
        }
    }

    // Validate individual products
    for product in &product_group.products {
        match validate_create_product(_action.clone(), product.clone())? {
            ValidateCallbackResult::Valid => (),
            invalid => return Ok(invalid),
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_product(
    _action: Update,
    _product: Product,
    _original_action: EntryCreationAction,
    _original_product: Product,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Products cannot be updated".to_string(),
    ))
}

pub fn validate_update_product_group(
    _action: Update,
    _product_group: ProductGroup,
    _original_action: EntryCreationAction,
    _original_product_group: ProductGroup,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Product groups cannot be updated".to_string(),
    ))
}

pub fn validate_delete_product(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_product: Product,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Products cannot be deleted".to_string(),
    ))
}

pub fn validate_delete_product_group(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_product_group: ProductGroup,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Product groups cannot be deleted".to_string(),
    ))
}