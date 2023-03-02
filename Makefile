build:
	cargo build --release
action:build
	@cargo test
	@cargo run --example link RUST_LOG=info
	@RUST_LOG=info  cargo run --example attr
	@RUST_LOG=info  cargo run --example rename
	@RUST_LOG=info  cargo run --example delete
	@RUST_LOG=info  cargo run --example mfs

