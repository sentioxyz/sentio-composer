set -e

cd app && npm run start &

cd app/client && npm run serve &
cd ..
