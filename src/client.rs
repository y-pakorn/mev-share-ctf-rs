use mev_share_rpc_api::MevApiClient;

pub struct Client {
    pub inner: Box<dyn MevApiClient + Sync + Send>,
}

impl AsRef<dyn MevApiClient + Sync + Send> for Client {
    fn as_ref(&self) -> &(dyn MevApiClient + Send + Sync + 'static) {
        self.inner.as_ref()
    }
}
