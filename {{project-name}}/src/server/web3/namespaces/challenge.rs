use crate::server::web3::backend::into_rpc_error;
use crate::server::web3::namespaces::bridge::BridgeNamespace;
use bridge_rpc::error::Web3Error;
use jsonrpsee::core::{async_trait, RpcResult};
use types::challenger::{
    ChallengeStatus, QueryChallengeDataReponse, QueryChallengeDataRequest, StartChallengeResponse,
};
use types::pegout::{PegoutOperationRangeResponse, PegoutOperationRangerRequest};
use types::{
    operator::{Operator, OperatorFilter},
    pegin::PeginDetails,
    rpc::{PeginCreateRequest, PeginCreateResponse, PeginRequest},
};
// ) -> Result<ScriptBuf, Web3Error> {

impl BridgeNamespace {
    // 1. query pegout operation: [from_pegout_id, len], len<=50 (limited) order by created time
    //         response: PegoutOperationRangeResponse
    pub async fn query_pegout_operations_impl(
        &self,
        request: PegoutOperationRangerRequest,
    ) -> Result<Vec<PegoutOperationRangeResponse>, Web3Error> {
        self.state
            .bridge
            .query_pegout_operations_for_challenger(request)
            .await
            .map_err(|e| Web3Error::ChallengeError(e.to_string()))
    }

    // 2. query challenge data: tx and witness:
    //  2.1 query a. split proof witness data
    //  2.2 query pegout challenger tx.
    pub async fn query_challenge_data_impl(
        &self,
        request: QueryChallengeDataRequest,
    ) -> Result<QueryChallengeDataReponse, Web3Error> {
        self.state
            .bridge
            .query_challenge_data(request)
            .await
            .map_err(|e| Web3Error::ChallengeError(e.to_string()))
    }

    pub async fn start_challenge_impl(
        &self,
        request: StartChallengeResponse,
    ) -> Result<u32, Web3Error> {
        self.state
            .bridge
            .submit_challenge(request)
            .await
            .map_err(|e| Web3Error::ChallengeError(e.to_string()))
    }

    pub async fn query_challenge_status_impl(
        &self,
        challenge_id: u32,
    ) -> Result<ChallengeStatus, Web3Error> {
        self.state
            .bridge
            .query_challenge_status(challenge_id)
            .await
            .map_err(|e| Web3Error::ChallengeError(e.to_string()))
    }
}
