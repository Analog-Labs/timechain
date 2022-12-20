use sp_inherents::{InherentIdentifier, InherentDataProvider, InherentData, Error};

/// ID of inherent data we submit to runtime
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"tsskey01";
/// TSS Public key output type
pub type TimeTssKey = [u8; 32];

/// Our inherent data provider for runtime
pub struct TimeInherentTssDataProvider;

#[async_trait::async_trait]
impl InherentDataProvider for TimeInherentTssDataProvider {
    fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
        todo!()
    }

    async fn try_handle_error(&self, identifier: &InherentIdentifier, error: &[u8]) -> Option<Result<(), Error>> {
        todo!()
    }
}