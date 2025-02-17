mod common;

use common::TestEnv;

#[tokio::test]
// Resembles tc-cli smoke test
async fn smoke() {
	let _env = TestEnv::spawn().await.expect("Failed to spawn Test Environment");

	todo!()
}
