//! Provides a builder API for creating and sending Sequencer REST requests.
//!
//! This builder utilises a type state builder pattern with generics to only allow valid operations at each stage of the build process.
//! Each stage is consumed to generate the next stage and the final stage executes the query.
//!
//! Here is an overview of the five builder stages.
//!
//!   1. [Init](stage::Init) which provides the entry point of the [builder](Request).
//!   2. [Gateway](stage::Gateway) where you select between the read and write gateways.
//!   3. [Method](stage::Method) where you select the REST API method.
//!   4. [Params](stage::Params) where you select the retry behavior.
//!   5. [Final](stage::Final) where you select the REST operation type, which is then executed.

use crate::{
    core::{ClassHash, ContractAddress, StarknetTransactionHash, StorageAddress},
    sequencer::error::SequencerError,
};

/// A Sequencer Request builder.
pub struct Request<'a, S: RequestState> {
    state: S,
    url: reqwest::Url,
    client: &'a reqwest::Client,
}

/// Describes the retry behavior of a [Request] and is specified using
#[allow(dead_code)]
pub enum Retry {
    Enabled,
    Disabled,
}

pub mod stage {
    /// Provides the [builder](super::Request::builder) entry-point.
    pub struct Init;

    /// Select between the [read](super::Request::feeder_gateway) and [write](super::Request::gateway) Sequencer gateways.
    pub struct Gateway;

    /// Select the Sequencer API method to call:
    /// - [add_transaction](super::Request::add_transaction)
    /// - [call_contract](super::Request::call_contract)
    /// - [get_block](super::Request::get_block)
    /// - [get_full_contract](super::Request::get_full_contract)
    /// - [get_class_by_hash](super::Request::get_class_by_hash)
    /// - [get_class_hash_at](super::Request::get_class_hash_at)
    /// - [get_storage_at](super::Request::get_storage_at)
    /// - [get_transaction](super::Request::get_transaction)
    /// - [get_transaction_status](super::Request::get_transaction_status)
    /// - [get_state_update](super::Request::get_state_update)
    /// - [get_contract_addresses](super::Request::get_contract_addresses)
    pub struct Method;

    /// Specify the request parameters:
    /// - [at_block](super::Request::with_block)
    /// - [with_contract_address](super::Request::with_contract_address)
    /// - [with_class_hash](super::Request::with_class_hash)
    /// - [with_optional_token](super::Request::with_optional_token)
    /// - [with_storage_address](super::Request::with_storage_address)
    /// - [with_transaction_hash](super::Request::with_transaction_hash)
    /// - [add_param](super::Request::add_param) (allows adding custom (name, value) parameter)
    ///
    /// and then specify the [retry behavior](super::Request::with_retry).
    pub struct Params;

    /// Specify the REST operation send the request:
    /// - [get](super::Request::get)
    /// - [get_as_bytes](super::Request::get_as_bytes)
    /// - [post_with_json](super::Request::post_with_json)
    pub struct Final {
        pub retry: super::Retry,
    }

    impl super::RequestState for Init {}
    impl super::RequestState for Gateway {}
    impl super::RequestState for Method {}
    impl super::RequestState for Params {}
    impl super::RequestState for Final {}
}

impl<'a> Request<'a, stage::Init> {
    /// Initialize a [Request] builder.
    pub fn builder(client: &'a reqwest::Client, url: reqwest::Url) -> Request<'a, stage::Gateway> {
        Request {
            url,
            client,
            state: stage::Gateway,
        }
    }
}

impl<'a> Request<'a, stage::Gateway> {
    /// The Sequencer write gateway, typically only used for submitting StarkNet transactions.
    pub fn gateway(self) -> Request<'a, stage::Method> {
        self.with_gateway("gateway")
    }

    /// The Sequencer read gateway, used for all queries which are not submitting transactions.
    pub fn feeder_gateway(self) -> Request<'a, stage::Method> {
        self.with_gateway("feeder_gateway")
    }

    fn with_gateway(mut self, gateway: &str) -> Request<'a, stage::Method> {
        self.url
            .path_segments_mut()
            .expect("Base URL is valid")
            .push(gateway);
        Request {
            url: self.url,
            client: self.client,
            state: stage::Method,
        }
    }
}

impl<'a> Request<'a, stage::Method> {
    pub fn add_transaction(self) -> Request<'a, stage::Params> {
        self.with_method("add_transaction")
    }

    pub fn call_contract(self) -> Request<'a, stage::Params> {
        self.with_method("call_contract")
    }

    pub fn get_block(self) -> Request<'a, stage::Params> {
        self.with_method("get_block")
    }

    pub fn get_full_contract(self) -> Request<'a, stage::Params> {
        self.with_method("get_full_contract")
    }

    pub fn get_class_by_hash(self) -> Request<'a, stage::Params> {
        self.with_method("get_class_by_hash")
    }

    pub fn get_class_hash_at(self) -> Request<'a, stage::Params> {
        self.with_method("get_class_hash_at")
    }

    pub fn get_storage_at(self) -> Request<'a, stage::Params> {
        self.with_method("get_storage_at")
    }

    pub fn get_transaction(self) -> Request<'a, stage::Params> {
        self.with_method("get_transaction")
    }

    pub fn get_transaction_status(self) -> Request<'a, stage::Params> {
        self.with_method("get_transaction_status")
    }

    pub fn get_state_update(self) -> Request<'a, stage::Params> {
        self.with_method("get_state_update")
    }

    pub fn get_contract_addresses(self) -> Request<'a, stage::Params> {
        self.with_method("get_contract_addresses")
    }

    /// Appends the given method to the request url.
    fn with_method(mut self, method: &str) -> Request<'a, stage::Params> {
        self.url
            .path_segments_mut()
            .expect("Base URL is valid")
            .push(method);

        Request {
            url: self.url,
            client: self.client,
            state: stage::Params,
        }
    }
}

impl<'a> Request<'a, stage::Params> {
    pub fn with_block<B: Into<crate::core::BlockId>>(self, block: B) -> Self {
        use crate::core::BlockId;
        use std::borrow::Cow;

        let block: BlockId = block.into();
        let (name, value) = match block {
            BlockId::Number(number) => ("blockNumber", Cow::from(number.0.to_string())),
            BlockId::Hash(hash) => ("blockHash", hash.0.to_hex_str()),
            // These have to use "blockNumber", "blockHash" does not accept tags.
            BlockId::Latest => ("blockNumber", Cow::from("latest")),
            BlockId::Pending => ("blockNumber", Cow::from("pending")),
        };

        self.add_param(name, &value)
    }

    pub fn with_contract_address(self, address: ContractAddress) -> Self {
        self.add_param("contractAddress", &address.0.to_hex_str())
    }

    pub fn with_class_hash(self, class_hash: ClassHash) -> Self {
        self.add_param("classHash", &class_hash.0.to_hex_str())
    }

    pub fn with_optional_token(self, token: Option<&str>) -> Self {
        match token {
            Some(token) => self.add_param("token", token),
            None => self,
        }
    }

    pub fn with_storage_address(self, address: StorageAddress) -> Self {
        use crate::rpc::serde::starkhash_to_dec_str;
        self.add_param("key", &starkhash_to_dec_str(&address.0))
    }

    pub fn with_transaction_hash(self, hash: StarknetTransactionHash) -> Self {
        self.add_param("transactionHash", &hash.0.to_hex_str())
    }

    pub fn add_param(mut self, name: &str, value: &str) -> Self {
        self.url.query_pairs_mut().append_pair(name, value);
        self
    }

    /// Sets the request retry behavior.
    pub fn with_retry(self, retry: Retry) -> Request<'a, stage::Final> {
        Request {
            url: self.url,
            client: self.client,
            state: stage::Final { retry },
        }
    }
}

impl<'a> Request<'a, stage::Final> {
    /// Sends the Sequencer request as a REST `GET` operation and parses the response into `T`.
    pub async fn get<T>(self) -> Result<T, SequencerError>
    where
        T: serde::de::DeserializeOwned,
    {
        async fn send_request<T: serde::de::DeserializeOwned>(
            url: reqwest::Url,
            client: &reqwest::Client,
        ) -> Result<T, SequencerError> {
            let response = client.get(url).send().await?;
            parse::<T>(response).await
        }

        match self.state.retry {
            Retry::Disabled => send_request(self.url, self.client).await,
            Retry::Enabled => {
                retry0(
                    || async {
                        let clone_url = self.url.clone();
                        send_request(clone_url, self.client).await
                    },
                    retry_condition,
                )
                .await
            }
        }
    }

    /// Sends the Sequencer request as a REST `GET` operation and returns the response's bytes.
    pub async fn get_as_bytes(self) -> Result<bytes::Bytes, SequencerError> {
        async fn get_as_bytes_inner(
            url: reqwest::Url,
            client: &reqwest::Client,
        ) -> Result<bytes::Bytes, SequencerError> {
            let response = client.get(url).send().await?;
            let response = parse_raw(response).await?;
            let bytes = response.bytes().await?;
            Ok(bytes)
        }

        match self.state.retry {
            Retry::Disabled => get_as_bytes_inner(self.url, self.client).await,
            Retry::Enabled => {
                retry0(
                    || async {
                        let clone_url = self.url.clone();
                        get_as_bytes_inner(clone_url, self.client).await
                    },
                    retry_condition,
                )
                .await
            }
        }
    }

    /// Sends the Sequencer request as a REST `POST` operation, in addition to the specified
    /// JSON body. The response is parsed as type `T`.
    pub async fn post_with_json<T, J>(self, json: &J) -> Result<T, SequencerError>
    where
        T: serde::de::DeserializeOwned,
        J: serde::Serialize + ?Sized,
    {
        async fn post_with_json_inner<T, J>(
            url: reqwest::Url,
            client: &reqwest::Client,
            json: &J,
        ) -> Result<T, SequencerError>
        where
            T: serde::de::DeserializeOwned,
            J: serde::Serialize + ?Sized,
        {
            let response = client.post(url).json(json).send().await?;
            parse::<T>(response).await
        }

        match self.state.retry {
            Retry::Disabled => post_with_json_inner(self.url, self.client, json).await,
            Retry::Enabled => {
                retry0(
                    || async {
                        let clone_url = self.url.clone();
                        post_with_json_inner(clone_url, self.client, json).await
                    },
                    retry_condition,
                )
                .await
            }
        }
    }
}

async fn parse<T>(response: reqwest::Response) -> Result<T, SequencerError>
where
    T: ::serde::de::DeserializeOwned,
{
    let response = parse_raw(response).await?;
    // Attempt to deserialize the actual data we are looking for
    let response = response.json::<T>().await?;
    Ok(response)
}

/// Helper function which allows skipping deserialization when required.
async fn parse_raw(response: reqwest::Response) -> Result<reqwest::Response, SequencerError> {
    use crate::sequencer::error::StarknetError;

    // Starknet specific errors end with a 500 status code
    // but the body contains a JSON object with the error description
    if response.status() == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
        let starknet_error = response.json::<StarknetError>().await?;
        return Err(SequencerError::StarknetError(starknet_error));
    }
    // Status codes 400..499 and 501..599 are mapped to SequencerError::TransportError
    response.error_for_status_ref().map(|_| ())?;
    Ok(response)
}

pub trait RequestState {}

/// Wrapper function to allow retrying sequencer queries in an exponential manner.
async fn retry0<T, Fut, FutureFactory, Ret>(
    future_factory: FutureFactory,
    retry_condition: Ret,
) -> Result<T, SequencerError>
where
    Fut: futures::Future<Output = Result<T, SequencerError>>,
    FutureFactory: FnMut() -> Fut,
    Ret: FnMut(&SequencerError) -> bool,
{
    use crate::retry::Retry;
    use std::num::NonZeroU64;

    Retry::exponential(future_factory, NonZeroU64::new(2).unwrap())
        .factor(NonZeroU64::new(15).unwrap())
        .max_delay(std::time::Duration::from_secs(30))
        .when(retry_condition)
        .await
}

/// Determines if an error is retryable or not.
fn retry_condition(e: &SequencerError) -> bool {
    use reqwest::StatusCode;
    use tracing::{debug, error, info, warn};

    match e {
        SequencerError::ReqwestError(e) => {
            if e.is_body() || e.is_connect() || e.is_timeout() {
                info!(reason=%e, "Request failed, retrying");
            } else if e.is_status() {
                match e.status() {
                    Some(
                        StatusCode::NOT_FOUND
                        | StatusCode::TOO_MANY_REQUESTS
                        | StatusCode::BAD_GATEWAY
                        | StatusCode::SERVICE_UNAVAILABLE
                        | StatusCode::GATEWAY_TIMEOUT,
                    ) => {
                        debug!(reason=%e, "Request failed, retrying");
                    }
                    Some(StatusCode::INTERNAL_SERVER_ERROR) => {
                        error!(reason=%e, "Request failed, retrying");
                    }
                    Some(_) => warn!(reason=%e, "Request failed, retrying"),
                    None => unreachable!(),
                }
            } else if e.is_decode() {
                error!(reason=%e, "Request failed, retrying");
            } else {
                warn!(reason=%e, "Request failed, retrying");
            }

            true
        }
        SequencerError::StarknetError(_) => false,
    }
}

#[cfg(test)]
mod tests {
    mod retry {
        use assert_matches::assert_matches;
        use http::{response::Builder, StatusCode};
        use pretty_assertions::assert_eq;
        use std::{
            collections::VecDeque, convert::Infallible, net::SocketAddr, sync::Arc, time::Duration,
        };
        use tokio::{sync::Mutex, task::JoinHandle};
        use warp::Filter;

        use crate::sequencer::builder::{retry0, retry_condition};

        // A test helper
        fn status_queue_server(
            statuses: VecDeque<(StatusCode, &'static str)>,
        ) -> (JoinHandle<()>, SocketAddr) {
            use std::cell::RefCell;

            let statuses = Arc::new(Mutex::new(RefCell::new(statuses)));
            let any = warp::any().and_then(move || {
                let s = statuses.clone();
                async move {
                    let s = s.lock().await;
                    let s = s.borrow_mut().pop_front().unwrap();
                    Result::<_, Infallible>::Ok(Builder::new().status(s.0).body(s.1))
                }
            });

            let (addr, run_srv) = warp::serve(any).bind_ephemeral(([127, 0, 0, 1], 0));
            let server_handle = tokio::spawn(run_srv);
            (server_handle, addr)
        }

        // A test helper
        fn slow_server() -> (tokio::task::JoinHandle<()>, std::net::SocketAddr) {
            async fn slow() -> Result<impl warp::Reply, Infallible> {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(Builder::new().status(200).body(""))
            }

            let any = warp::any().and_then(slow);
            let (addr, run_srv) = warp::serve(any).bind_ephemeral(([127, 0, 0, 1], 0));
            let server_handle = tokio::spawn(run_srv);
            (server_handle, addr)
        }

        #[test_log::test(tokio::test)]
        async fn stop_on_ok() {
            use crate::sequencer::builder;

            let statuses = VecDeque::from([
                (StatusCode::TOO_MANY_REQUESTS, ""),
                (StatusCode::BAD_GATEWAY, ""),
                (StatusCode::SERVICE_UNAVAILABLE, ""),
                (StatusCode::GATEWAY_TIMEOUT, ""),
                (StatusCode::OK, r#""Finally!""#),
                (StatusCode::TOO_MANY_REQUESTS, ""),
                (StatusCode::BAD_GATEWAY, ""),
                (StatusCode::SERVICE_UNAVAILABLE, ""),
            ]);

            let (_jh, addr) = status_queue_server(statuses);
            let result = retry0(
                || async {
                    let mut url = reqwest::Url::parse("http://localhost/").unwrap();
                    url.set_port(Some(addr.port())).unwrap();
                    let response = reqwest::get(url).await?;
                    builder::parse::<String>(response).await
                },
                retry_condition,
            )
            .await
            .unwrap();
            assert_eq!(result, "Finally!");
        }

        #[test_log::test(tokio::test)]
        async fn stop_on_fatal() {
            use crate::sequencer::builder;
            use crate::sequencer::error::{SequencerError, StarknetErrorCode};

            let statuses = VecDeque::from([
                (StatusCode::TOO_MANY_REQUESTS, ""),
                (StatusCode::BAD_GATEWAY, ""),
                (StatusCode::SERVICE_UNAVAILABLE, ""),
                (StatusCode::GATEWAY_TIMEOUT, ""),
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    r#"{"code":"StarknetErrorCode.BLOCK_NOT_FOUND","message":""}"#,
                ),
                (StatusCode::TOO_MANY_REQUESTS, ""),
                (StatusCode::BAD_GATEWAY, ""),
                (StatusCode::SERVICE_UNAVAILABLE, ""),
            ]);

            let (_jh, addr) = status_queue_server(statuses);
            let error = retry0(
                || async {
                    let mut url = reqwest::Url::parse("http://localhost/").unwrap();
                    url.set_port(Some(addr.port())).unwrap();
                    let response = reqwest::get(url).await?;
                    builder::parse::<String>(response).await
                },
                retry_condition,
            )
            .await
            .unwrap_err();
            assert_matches!(
                error,
                SequencerError::StarknetError(se) => assert_eq!(se.code, StarknetErrorCode::BlockNotFound)
            );
        }

        #[tokio::test(flavor = "current_thread", start_paused = true)]
        async fn request_timeout() {
            use crate::sequencer::builder;

            use std::sync::atomic::{AtomicUsize, Ordering};

            let (_jh, addr) = slow_server();
            static CNT: AtomicUsize = AtomicUsize::new(0);

            let fut = retry0(
                || async {
                    let mut url = reqwest::Url::parse("http://localhost/").unwrap();
                    url.set_port(Some(addr.port())).unwrap();

                    let client = reqwest::Client::builder().build().unwrap();

                    CNT.fetch_add(1, Ordering::Relaxed);

                    // This is the same as using Client::builder().timeout()
                    let response = client
                        .get(url)
                        .timeout(Duration::from_millis(1))
                        .send()
                        .await?;
                    builder::parse::<String>(response).await
                },
                retry_condition,
            );

            // The retry loops forever, so wrap it in a timeout and check the counter.
            tokio::time::timeout(Duration::from_millis(250), fut)
                .await
                .unwrap_err();
            // 4th try should have timedout if this is really exponential backoff
            assert_eq!(CNT.load(Ordering::Relaxed), 4);
        }
    }
}
