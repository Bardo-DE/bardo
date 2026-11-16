use super::*;
use crate::probe::testing::chat;
use account_capability::LlmFeature;

#[tokio::test]
async fn with_no_key_store_every_report_is_unreachable() {
    let claims = chat("m", "m", 2048, &[LlmFeature::Chat]).claims;
    assert!(matches!(
        NoReports
            .report(Runtime::Ollama, &claims, Standing::Online)
            .await,
        Err(ReportFault::Unreachable)
    ));
}
