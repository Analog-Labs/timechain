fn main() {
	let status = std::process::Command::new("forge")
		.current_dir("gateway")
		.arg("build")
		.status()
		.expect("failed to run forge");
	assert!(status.success(), "forge exited with code 1");
}
