use hdk::prelude::*;

// Concurrent record retrieval function (kept from original implementation)

pub fn concurrent_get_records(hashes: Vec<ActionHash>) -> ExternResult<Vec<Record>> {
    const BATCH_SIZE: usize = 200;
    let mut all_records = Vec::new();
    let mut _successful_fetches = 0;
    let mut _failed_fetches = 0;

    for (_i, batch) in hashes.chunks(BATCH_SIZE).enumerate() {
        let input: Vec<_> = batch
            .iter()
            .map(|hash| {
                GetInput::new(hash.clone().into(), GetOptions::network())
            })
            .collect();

        match HDK.with(|hdk| hdk.borrow().get(input)) {
            Ok(records) => {
                let successful_count = records.iter().filter(|r| r.is_some()).count();
                let failed_count = records.len() - successful_count;
                
                _successful_fetches += successful_count;
                _failed_fetches += failed_count;
                
                // Log details about failed fetches
                if failed_count > 0 {
                    for (_j, record_opt) in records.iter().enumerate() {
                        if record_opt.is_none() {
                        }
                    }
                }
                
                all_records.extend(records.into_iter().flatten());
            },
            Err(e) => {
                _failed_fetches += batch.len();
                return Err(e);
            }
        }
    }
    
    Ok(all_records)
}
