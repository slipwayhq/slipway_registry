build:
  cargo build --release

test: build
  cargo test --release

deploy:
  fly deploy
