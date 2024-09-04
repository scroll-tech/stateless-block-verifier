use crate::TxTrace;
use alloy::{
    consensus::{Transaction, TxEnvelope, TxType},
    eips::eip2718::Encodable2718,
    eips::{eip2930::AccessList, eip7702::SignedAuthorization},
    primitives::{Address, Bytes, ChainId, Signature, SignatureError, TxKind, B256, U256, U64},
    rlp::{BufMut, BytesMut, Encodable, Header},
};
use revm_primitives::TxEnv;
use serde_with::{serde_as, DefaultOnNull};

/// Wrapped Ethereum Transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedTransaction {
    /// Normal enveloped ethereum transaction
    Enveloped(TxEnvelope),
    /// Layer1 Message Transaction
    L1Msg(TxL1Msg),
}

/// Layer1 Message Transaction
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TxL1Msg {
    /// The 32-byte hash of the transaction.
    pub tx_hash: B256,
    /// The 160-bit address of the message call’s sender.
    pub from: Address,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    pub nonce: u64,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    pub gas_limit: u128,
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    pub to: TxKind,
    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    pub value: U256,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
}

/// Transaction Trace
#[serde_as]
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, serde::Deserialize, Default, Debug, Clone,
)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug, Hash, PartialEq, Eq))]
pub struct TransactionTrace {
    /// tx hash
    #[serde(default, rename = "txHash")]
    pub tx_hash: B256,
    /// tx type (in raw from)
    #[serde(rename = "type")]
    pub ty: u8,
    /// nonce
    pub nonce: u64,
    /// gas limit
    pub gas: u64,
    #[serde(rename = "gasPrice")]
    /// gas price
    pub gas_price: U256,
    #[serde(rename = "gasTipCap")]
    /// gas tip cap
    pub gas_tip_cap: Option<U256>,
    #[serde(rename = "gasFeeCap")]
    /// gas fee cap
    pub gas_fee_cap: Option<U256>,
    /// from
    pub from: Address,
    /// to, NONE for creation (0 addr)
    pub to: Option<Address>,
    /// chain id
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    /// value amount
    pub value: U256,
    /// call data
    pub data: Bytes,
    /// is creation
    #[serde(rename = "isCreate")]
    pub is_create: bool,
    /// access list
    #[serde(rename = "accessList")]
    #[serde_as(as = "DefaultOnNull")]
    pub access_list: AccessList,
    /// signature v
    pub v: U64,
    /// signature r
    pub r: U256,
    /// signature s
    pub s: U256,
}

impl TxTrace for TransactionTrace {
    fn tx_hash(&self) -> B256 {
        self.tx_hash
    }

    fn ty(&self) -> u8 {
        self.ty
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u128 {
        self.gas as u128
    }

    fn gas_price(&self) -> u128 {
        self.gas_price.to()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.gas_fee_cap.map(|v| v.to()).unwrap_or_default()
    }

    fn max_priority_fee_per_gas(&self) -> u128 {
        self.gas_tip_cap.map(|v| v.to()).unwrap_or_default()
    }

    unsafe fn get_from_unchecked(&self) -> Address {
        self.from
    }

    fn to(&self) -> TxKind {
        if self.is_create {
            TxKind::Create
        } else {
            debug_assert!(self.to.map(|a| !a.is_zero()).unwrap_or(false));
            TxKind::Call(self.to.expect("to address must be present"))
        }
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id.to()
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn data(&self) -> Bytes {
        self.data.clone()
    }

    fn access_list(&self) -> AccessList {
        self.access_list.clone()
    }

    fn signature(&self) -> Result<Signature, SignatureError> {
        Signature::from_rs_and_parity(self.r, self.s, self.v)
    }
}

impl TxTrace for ArchivedTransactionTrace {
    fn tx_hash(&self) -> B256 {
        self.tx_hash
    }

    fn ty(&self) -> u8 {
        self.ty
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u128 {
        self.gas as u128
    }

    fn gas_price(&self) -> u128 {
        self.gas_price.to()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.gas_fee_cap
            .as_ref()
            .map(|v| v.to())
            .unwrap_or_default()
    }

    fn max_priority_fee_per_gas(&self) -> u128 {
        self.gas_tip_cap
            .as_ref()
            .map(|v| v.to())
            .unwrap_or_default()
    }

    unsafe fn get_from_unchecked(&self) -> Address {
        self.from
    }

    fn to(&self) -> TxKind {
        if self.is_create {
            TxKind::Create
        } else {
            debug_assert!(self.to.as_ref().map(|a| !a.is_zero()).unwrap_or(false));
            TxKind::Call(*self.to.as_ref().expect("to address must be present"))
        }
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id.to()
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn data(&self) -> Bytes {
        Bytes::copy_from_slice(self.data.as_ref())
    }

    fn access_list(&self) -> AccessList {
        rkyv::Deserialize::<AccessList, _>::deserialize(&self.access_list, &mut rkyv::Infallible)
            .unwrap()
    }

    fn signature(&self) -> Result<Signature, SignatureError> {
        Signature::from_rs_and_parity(self.r, self.s, self.v)
    }
}

impl Transaction for TypedTransaction {
    fn chain_id(&self) -> Option<ChainId> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.chain_id(),
            TypedTransaction::L1Msg(tx) => tx.chain_id(),
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.nonce(),
            TypedTransaction::L1Msg(tx) => tx.nonce(),
        }
    }

    fn gas_limit(&self) -> u128 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.gas_limit(),
            TypedTransaction::L1Msg(tx) => tx.gas_limit(),
        }
    }

    fn gas_price(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.gas_price(),
            TypedTransaction::L1Msg(tx) => tx.gas_price(),
        }
    }

    fn max_fee_per_gas(&self) -> u128 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_fee_per_gas(),
            TypedTransaction::L1Msg(tx) => tx.max_fee_per_gas(),
        }
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_priority_fee_per_gas(),
            TypedTransaction::L1Msg(tx) => tx.max_priority_fee_per_gas(),
        }
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_fee_per_blob_gas(),
            TypedTransaction::L1Msg(tx) => tx.max_fee_per_blob_gas(),
        }
    }

    fn priority_fee_or_price(&self) -> u128 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.priority_fee_or_price(),
            TypedTransaction::L1Msg(tx) => tx.priority_fee_or_price(),
        }
    }

    fn to(&self) -> TxKind {
        match self {
            TypedTransaction::Enveloped(tx) => tx.to(),
            TypedTransaction::L1Msg(tx) => tx.to(),
        }
    }

    fn value(&self) -> U256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.value(),
            TypedTransaction::L1Msg(tx) => tx.value(),
        }
    }

    fn input(&self) -> &[u8] {
        match self {
            TypedTransaction::Enveloped(tx) => tx.input(),
            TypedTransaction::L1Msg(tx) => tx.input(),
        }
    }

    fn ty(&self) -> u8 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.ty(),
            TypedTransaction::L1Msg(tx) => tx.ty(),
        }
    }

    fn access_list(&self) -> Option<&AccessList> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.access_list(),
            TypedTransaction::L1Msg(tx) => tx.access_list(),
        }
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.blob_versioned_hashes(),
            TypedTransaction::L1Msg(tx) => tx.blob_versioned_hashes(),
        }
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.authorization_list(),
            TypedTransaction::L1Msg(tx) => tx.authorization_list(),
        }
    }
}

impl TxL1Msg {
    /// Outputs the length of the transaction's fields.
    #[doc(hidden)]
    pub fn fields_len(&self) -> usize {
        let mut len = 0;
        len += self.nonce.length();
        len += self.gas_limit.length();
        len += self.to.length();
        len += self.value.length();
        len += self.input.0.length();
        len += self.from.length();
        len
    }
}

impl Transaction for TxL1Msg {
    fn chain_id(&self) -> Option<ChainId> {
        None
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u128 {
        self.gas_limit
    }

    fn gas_price(&self) -> Option<u128> {
        Some(0)
    }

    fn max_fee_per_gas(&self) -> u128 {
        0
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        None
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        None
    }

    fn priority_fee_or_price(&self) -> u128 {
        0
    }

    fn to(&self) -> TxKind {
        self.to
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn input(&self) -> &[u8] {
        self.input.as_ref()
    }

    fn ty(&self) -> u8 {
        0x7e
    }

    fn access_list(&self) -> Option<&AccessList> {
        None
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        None
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        None
    }
}

impl Encodable for TxL1Msg {
    fn encode(&self, out: &mut dyn BufMut) {
        self.nonce.encode(out);
        self.gas_limit.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.from.encode(out);
    }
}

impl Encodable2718 for TxL1Msg {
    fn type_flag(&self) -> Option<u8> {
        Some(0x7e)
    }

    fn encode_2718_len(&self) -> usize {
        let payload_length = self.fields_len();
        1 + Header {
            list: true,
            payload_length,
        }
        .length()
            + payload_length
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        0x7eu8.encode(out);
        let header = Header {
            list: true,
            payload_length: self.fields_len(),
        };
        header.encode(out);
        self.encode(out)
    }
}

impl TypedTransaction {
    /// Return the hash of the inner transaction.
    pub fn tx_hash(&self) -> &B256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.tx_hash(),
            TypedTransaction::L1Msg(tx) => &tx.tx_hash,
        }
    }

    /// Get the caller of the transaction, recover the signer if the transaction is enveloped.
    ///
    /// Fails if the transaction is enveloped and recovering the signer fails.
    pub fn get_or_recover_signer(&self) -> Result<Address, SignatureError> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.recover_signer(),
            TypedTransaction::L1Msg(tx) => Ok(tx.from),
        }
    }

    /// Encode the transaction according to [EIP-2718] rules. First a 1-byte
    /// type flag in the range 0x0-0x7f, then the body of the transaction.
    pub fn rlp(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        match self {
            TypedTransaction::Enveloped(tx) => tx.encode_2718(&mut bytes),
            TypedTransaction::L1Msg(tx) => tx.encode_2718(&mut bytes),
        }
        Bytes(bytes.freeze())
    }

    fn data(&self) -> Bytes {
        match self {
            TypedTransaction::Enveloped(tx) => match tx.tx_type() {
                TxType::Legacy => tx.as_legacy().unwrap().tx().input.clone(),
                TxType::Eip1559 => tx.as_eip1559().unwrap().tx().input.clone(),
                TxType::Eip2930 => tx.as_eip2930().unwrap().tx().input.clone(),
                _ => unimplemented!("unsupported tx type {:?}", tx.tx_type()),
            },
            TypedTransaction::L1Msg(tx) => tx.input.clone(),
        }
    }

    /// creates [`revm::primitives::TxEnv`]
    pub fn tx_env(&self) -> Result<TxEnv, SignatureError> {
        Ok(TxEnv {
            caller: self.get_or_recover_signer()?,
            gas_limit: self.gas_limit() as u64,
            gas_price: self
                .gas_price()
                .map(U256::from)
                .expect("gas price is required"),
            transact_to: self.to(),
            value: self.value(),
            data: self.data(),
            nonce: Some(self.nonce()),
            chain_id: self.chain_id(),
            access_list: self.access_list().cloned().unwrap_or_default().0,
            gas_priority_fee: self.max_priority_fee_per_gas().map(U256::from),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACE: &str = include_str!("../../../../testdata/mainnet_blocks/8370400.json");

    const L1_MSG_TX: &str = r#"{
      "type": 126,
      "nonce": 927290,
      "txHash": "0x51b8fb307fd5d240145854763b25529eb2266403e717844d2f106bcc8d4a6c2f",
      "gas": 180000,
      "gasPrice": "0x0",
      "gasTipCap": "0x0",
      "gasFeeCap": "0x0",
      "from": "0x7885bcbd5cecef1336b5300fb5186a12ddd8c478",
      "to": "0x781e90f1c8fc4611c9b7497c3b47f99ef6969cbc",
      "chainId": "0x0",
      "value": "0x0",
      "data": "0x8ef1332e000000000000000000000000a033ff09f2da45f0e9ae495f525363722df42b2a0000000000000000000000009ebf2f33526cd571f8b2ad312492cb650870cfd6000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e263a00000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e48431f5c1000000000000000000000000d9a442856c234a39a81a089c06451ebaa4306a72000000000000000000000000c4d46e8402f476f269c379677c99f18e22ea030e000000000000000000000000c830fe4df0775d1c6ce5541693cbf4210ceac2fb000000000000000000000000c830fe4df0775d1c6ce5541693cbf4210ceac2fb0000000000000000000000000000000000000000000000000214e8348c4f001600000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
      "isCreate": false,
      "accessList": null,
      "v": "0x0",
      "r": "0x0",
      "s": "0x0"
    }"#;

    #[test]
    fn test_transaction_trace_deserialize() {
        let trace = serde_json::from_str::<serde_json::Value>(TRACE).unwrap()["result"].clone();
        let txs = trace["transactions"].clone();
        for tx in txs.as_array().unwrap() {
            let tx: TransactionTrace = serde_json::from_value(tx.clone()).unwrap();
            let _ = tx.try_build_typed_tx().unwrap();
        }
    }

    #[test]
    fn test_rlp() {
        let trace = serde_json::from_str::<serde_json::Value>(TRACE).unwrap()["result"].clone();
        let txs: Vec<TransactionTrace> =
            serde_json::from_value(trace["transactions"].clone()).unwrap();

        let block: eth_types::l2_types::BlockTrace = serde_json::from_value(trace).unwrap();

        for (idx, (eth_tx, tx)) in block.transactions.into_iter().zip(txs).enumerate() {
            let eth_tx = eth_tx.to_eth_tx(
                block.header.hash,
                block.header.number,
                Some((idx as u64).into()),
                block.header.base_fee_per_gas,
            );
            let eth_rlp = eth_tx.rlp();

            let tx = tx.try_build_typed_tx().unwrap();
            let tx_rlp = tx.rlp();

            assert_eq!(eth_rlp.as_ref(), tx_rlp.as_ref(), "tx: {}", idx);
        }

        let eth_l1_msg_tx: eth_types::l2_types::TransactionTrace =
            serde_json::from_str(L1_MSG_TX).unwrap();
        let eth_l1_msg_tx = eth_l1_msg_tx.to_eth_tx(None, None, None, Some(0x2605c9c.into()));
        let l1_msg_tx: TransactionTrace = serde_json::from_str(L1_MSG_TX).unwrap();
        let l1_msg_tx = l1_msg_tx.try_build_typed_tx().unwrap();

        let eth_rlp = eth_l1_msg_tx.rlp();
        let tx_rlp = l1_msg_tx.rlp();

        assert_eq!(eth_rlp.as_ref(), tx_rlp.as_ref());
    }
}
