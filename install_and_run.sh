cargo build -p example-plugin --target wasm32-unknown-unknown && cargo install --path crates/example-runner --force && mdbook build example-book