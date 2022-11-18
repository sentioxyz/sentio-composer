FROM node:bullseye

# Create app directory
WORKDIR /app

RUN mkdir -p /app/bin && curl -SL https://github.com/sentioxyz/sentio-composer/releases/download/v0.2.0/view-function-x86_64-unknown-linux-gnu.tar.gz | tar xvzf - -C /app/bin

# RUN mkdir bin && mv view-function ./bin 

COPY package*.json ./
COPY tsconfig.json .
COPY index.ts .

RUN npm install
RUN npm run compile

FROM node:bullseye
WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production
COPY --from=0 /app/dist .
COPY --from=0 /app/bin .
RUN npm install pm2 -g

EXPOSE 4000
CMD [ "pm2-runtime", "index.js" ]
