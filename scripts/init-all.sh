# build the CLI tool
echo "Build the CLI"
cargo build
# install dependencies for the server
echo "Install dependencies for nodejs server"
cd app/server && npm install && cd ..
# install dependencies for the frontend
echo "Install dependencies for the frontend"
cd client && npm install && cd ..
