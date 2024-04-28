use eth_types::{l2_types::StorageTrace, Address, Word};

pub(crate) fn collect_account_proofs(
    storage_trace: &StorageTrace,
) -> impl Iterator<Item = (&Address, impl IntoIterator<Item = &[u8]>)> + Clone {
    storage_trace.proofs.iter().flat_map(|kv_map| {
        kv_map
            .iter()
            .map(|(k, bts)| (k, bts.iter().map(|b| b.as_ref())))
    })
}

pub(crate) fn collect_storage_proofs(
    storage_trace: &StorageTrace,
) -> impl Iterator<Item = (&Address, &Word, impl IntoIterator<Item = &[u8]>)> + Clone {
    storage_trace.storage_proofs.iter().flat_map(|(k, kv_map)| {
        kv_map
            .iter()
            .map(move |(sk, bts)| (k, sk, bts.iter().map(|b| b.as_ref())))
    })
}
