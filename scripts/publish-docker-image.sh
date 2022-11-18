set -e

cd app/server

npm run docker

docker image push poytr1/sentio-composer-server
