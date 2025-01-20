use crate::server::web3::{backend::into_rpc_error, namespaces::bridge::BridgeNamespace};
use bitcoin::ScriptBuf;
use bridge_rpc::namespaces::bridge::BridgeNamespaceServer;
use jsonrpsee::core::{async_trait, RpcResult};
use types::challenger::{
    ChallengeStatus, QueryChallengeDataReponse, QueryChallengeDataRequest, StartChallengeResponse,
};
use types::pegin::{PeginEventDetails, PeginOperation};
use types::pegout::{
    PegoutDetail, PegoutEventDetail, PegoutInfo, PegoutOperationRangeResponse,
    PegoutOperationRangerRequest,
};
use types::pubsub::{
    ChallengeCommitteeTaskResponse, OperatorPegoutTxRequest, PeginCommitteeTaskResponse,
    PeginOperatorTaskResponse, PeginReceivedRequest, PegoutOperatorTaskResponse,
    PegoutReceivedRequest,
};
use types::rpc::{
    LPPeginRequest, LPPegoutRequest, PeginEventRangeRequest, PeginRequestFrontend,
    PeginTakeTxMsgReponse, PeginTakeTxMsgRequest, PegoutRequest, PegouttEventRangeRequest,
};
use types::{
    operator::{Operator, OperatorFilter},
    pegin::PeginDetails,
    rpc::{PeginCreateRequest, PeginCreateResponse, PeginRequest},
};

#[async_trait]
impl BridgeNamespaceServer for BridgeNamespace {
    async fn get_pegin_multi_sig_script(&self, pubkey: &str) -> RpcResult<ScriptBuf> {
        self.get_pegin_multi_sig_script_impl(pubkey)
            .await
            .map_err(into_rpc_error)
    }

    async fn query_operators(&self, filter: OperatorFilter) -> RpcResult<Vec<Operator>> {
        self.query_operators_impl(filter)
            .await
            .map_err(into_rpc_error)
    }

    async fn query_pegin_take_tx_sign_msg(
        &self,
        request: PeginTakeTxMsgRequest,
    ) -> RpcResult<PeginTakeTxMsgReponse> {
        self.query_pegin_take_tx_sign_msg_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    async fn create_pegin(
        &self,
        pegin_create_req: PeginCreateRequest,
    ) -> RpcResult<PeginCreateResponse> {
        self.create_pegin_impl(pegin_create_req)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_pegin(&self, pegin_req: PeginRequest) -> RpcResult<u32> {
        self.submit_pegin_impl(pegin_req)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_lp_pegin(&self, pegin_req: LPPeginRequest) -> RpcResult<u32> {
        self.submit_lp_pegin_impl(pegin_req)
            .await
            .map_err(into_rpc_error)
    }

    async fn query_pegin_details(&self, pegin_id: u32) -> RpcResult<PeginDetails> {
        self.query_pegin_details_impl(pegin_id)
            .await
            .map_err(into_rpc_error)
    }

    async fn query_pegout_detail(&self, pegin_id: u32) -> RpcResult<PegoutDetail> {
        self.query_pegout_detail_by_id_impl(pegin_id)
            .await
            .map_err(into_rpc_error)
    }

    async fn get_pegout_detail_by_burn_tx(&self, burn_tx_hash: String) -> RpcResult<PegoutDetail> {
        self.query_pegout_detail_by_burn_tx_hash_impl(burn_tx_hash)
            .await
            .map_err(into_rpc_error)
    }

    async fn get_pegout_history(&self, request: PegoutRequest) -> RpcResult<Vec<PegoutInfo>> {
        self.get_pegout_history_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    async fn get_pegin_history(&self, address: &str) -> RpcResult<Vec<PeginDetails>> {
        self.get_pegin_history_impl(address)
            .await
            .map_err(into_rpc_error)
    }

    async fn get_avaliable_pegout_amount(&self, amount: u64, n: u32) -> RpcResult<Vec<u64>> {
        self.get_avaliable_pegout_amount_impl(amount, n)
            .await
            .map_err(into_rpc_error)
    }

    async fn query_lp_pending_pegouts(&self) -> RpcResult<Vec<PegoutInfo>> {
        self.query_lp_pending_pegouts_impl()
            .await
            .map_err(into_rpc_error)
    }

    async fn try_lock_pegin_utxo(
        &self,
        request: LPPegoutRequest,
    ) -> RpcResult<Option<PeginOperation>> {
        self.try_lock_pegin_utxo_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    ///////////////////////////////////////////////////
    /////////////// Pegin/Pegout events.
    ///////////////////////////////////////////////////
    async fn query_pegin_events_by_range(
        &self,
        request: PeginEventRangeRequest,
    ) -> RpcResult<Vec<PeginEventDetails>> {
        self.query_pegin_events_by_range_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    async fn query_pegout_events_by_range(
        &self,
        request: PegouttEventRangeRequest,
    ) -> RpcResult<Vec<PegoutEventDetail>> {
        self.query_pegout_events_by_range_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    //////////////////////////////////////////////
    /////////   challenger related RPC:
    //////////////////////////////////////////////
    // 1. query pegout operation: [from_pegout_id, len], len<=50 (limited) order by created time
    //         response: PegoutOperationRangeResponse
    async fn query_pegout_operations(
        &self,
        request: PegoutOperationRangerRequest,
    ) -> RpcResult<Vec<PegoutOperationRangeResponse>> {
        {
            self.query_pegout_operations_impl(request)
                .await
                .map_err(into_rpc_error)
        }
    }

    // 2. query challenge data: tx and witness:
    //  2.1 query a. split proof witness data
    //  2.2 query pegout challenger tx.
    async fn query_challenge_data(
        &self,
        request: QueryChallengeDataRequest,
    ) -> RpcResult<QueryChallengeDataReponse> {
        self.query_challenge_data_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    async fn start_challenge(&self, request: StartChallengeResponse) -> RpcResult<u32> {
        self.start_challenge_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    async fn query_challenge_status(&self, challenge_id: u32) -> RpcResult<ChallengeStatus> {
        self.query_challenge_status_impl(challenge_id)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_pegin_operator_presigned_transactions(
        &self,
        presigned_txs: PeginOperatorTaskResponse,
    ) -> RpcResult<()> {
        self.submit_pegin_operator_presigned_transactions_impl(presigned_txs)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_pegout_operator_response(
        &self,
        pegout_response: PegoutOperatorTaskResponse,
    ) -> RpcResult<()> {
        self.submit_pegout_operator_response_impl(pegout_response)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_operator_pegout_tx(
        &self,
        pegout_request: OperatorPegoutTxRequest,
    ) -> RpcResult<()> {
        self.submit_operator_pegout_tx_impl(pegout_request)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_pegout_received(&self, request: PegoutReceivedRequest) -> RpcResult<()> {
        self.submit_pegout_received_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_pegin_received(&self, request: PeginReceivedRequest) -> RpcResult<()> {
        self.submit_pegin_received_impl(request)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_pegin_committee_presigned_transactions(
        &self,
        presigned_txs: PeginCommitteeTaskResponse,
    ) -> RpcResult<()> {
        self.submit_pegin_committee_presigned_transactions_impl(presigned_txs)
            .await
            .map_err(into_rpc_error)
    }

    async fn submit_challenge_committee_presigned_transactions(
        &self,
        response: ChallengeCommitteeTaskResponse,
    ) -> RpcResult<()> {
        self.submit_challenge_committee_presigned_transactions_impl(response)
            .await
            .map_err(into_rpc_error)
    }
}
