# build the CLI tool
cargo build
# install dependencies for the server
cd app && npm install
# install dependencies for the front
cd ../client && npm install
