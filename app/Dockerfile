FROM node:bullseye
# build client
WORKDIR /app

COPY ./client .
RUN npm run build-app


FROM node:bullseye

# Create app directory
WORKDIR /app

RUN mkdir -p /app/bin && curl -SL https://github.com/sentioxyz/sentio-composer/releases/latest/download/view-function-x86_64-unknown-linux-gnu.tar.gz | tar xvzf - -C /app/bin

COPY ./server/package*.json ./
COPY ./server/tsconfig.json .
COPY ./server/index.ts .

RUN npm install
RUN npm run compile

FROM node:bullseye
WORKDIR /app

COPY ./server/package*.json ./
RUN npm ci --only=production
COPY --from=0 /app/dist /app/public
COPY --from=1 /app/dist .
COPY --from=1 /app/bin .
RUN npm install pm2 -g

EXPOSE 4000
CMD [ "pm2-runtime", "index.js" ]
