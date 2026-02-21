.PHONY: test lint test-frontend test-backend lint-frontend lint-backend

test: test-frontend test-backend

test-frontend:
	bun run test:coverage

test-backend:
	cargo test --manifest-path src-tauri/Cargo.toml

lint: lint-frontend lint-backend

lint-frontend:
	bun run lint

lint-backend:
	cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
	cargo check --manifest-path src-tauri/Cargo.toml --all-targets
