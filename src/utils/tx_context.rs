use tokio::sync::oneshot;
use ton_types::UInt256;

pub trait ReadFromTransaction: Sized {
    fn read_from_transaction(ctx: &TxContext<'_>, state: HandleTransactionStatusTx)
        -> Option<Self>;
}

#[derive(Copy, Clone)]
pub struct TxContext<'a> {
    pub shard_accounts: &'a ton_block::ShardAccounts,
    pub block_info: &'a ton_block::BlockInfo,
    pub block_hash: &'a UInt256,
    pub account: &'a UInt256,
    pub transaction_hash: &'a UInt256,
    pub transaction_info: &'a ton_block::TransactionDescrOrdinary,
    pub transaction: &'a ton_block::Transaction,
    pub in_msg: &'a ton_block::Message,
    pub token_transaction: &'a Option<nekoton::core::models::TokenWalletTransaction>,
}

impl TxContext<'_> {
    pub fn in_msg_internal(&self) -> Option<&ton_block::Message> {
        if matches!(
            self.in_msg.header(),
            ton_block::CommonMsgInfo::IntMsgInfo(_)
        ) {
            Some(self.in_msg)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn in_msg_external(&self) -> Option<&ton_block::Message> {
        if matches!(
            self.in_msg.header(),
            ton_block::CommonMsgInfo::ExtInMsgInfo(_)
        ) {
            Some(self.in_msg)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn find_function_output(
        &self,
        function: &ton_abi::Function,
    ) -> Option<Vec<ton_abi::Token>> {
        let mut result = None;
        self.transaction
            .out_msgs
            .iterate(|ton_block::InRefValue(message)| {
                // Skip all messages except external outgoing
                if !matches!(message.header(), ton_block::CommonMsgInfo::ExtOutMsgInfo(_)) {
                    return Ok(true);
                }

                // Handle body if it exists
                let body = match message.body() {
                    Some(body) => body,
                    None => return Ok(true),
                };

                let function_id = nekoton_abi::read_function_id(&body)?;
                if function_id != function.output_id {
                    return Ok(true);
                }

                Ok(match function.decode_output(body, false) {
                    Ok(tokens) => {
                        result = Some(tokens);
                        false
                    }
                    Err(_) => true,
                })
            })
            .ok();
        result
    }

    #[allow(dead_code)]
    pub fn iterate_events<F>(&self, mut f: F)
    where
        F: FnMut(u32, ton_types::SliceData),
    {
        self.transaction
            .out_msgs
            .iterate(|ton_block::InRefValue(message)| {
                // Skip all messages except external outgoing
                if !matches!(message.header(), ton_block::CommonMsgInfo::ExtOutMsgInfo(_)) {
                    return Ok(true);
                }

                // Handle body if it exists
                let body = match message.body() {
                    Some(body) => body,
                    None => return Ok(true),
                };

                // Parse function id
                if let Ok(function_id) = nekoton_abi::read_function_id(&body) {
                    f(function_id, body)
                }

                // Process all messages
                Ok(true)
            })
            .ok();
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HandleTransactionStatus {
    Success,
    Fail,
}

pub type HandleTransactionStatusTx = oneshot::Sender<HandleTransactionStatus>;
pub type HandleTransactionStatusRx = oneshot::Receiver<HandleTransactionStatus>;
