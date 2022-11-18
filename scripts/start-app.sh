set -e

cd app/server && npm run start &

cd app/client && npm run serve &
cd ..
