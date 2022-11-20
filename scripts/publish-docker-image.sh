set -e

cd app

docker build -t poytr1/sentio-composer-app .
docker image push poytr1/sentio-composer-server
