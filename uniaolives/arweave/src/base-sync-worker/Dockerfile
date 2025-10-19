FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
ENV CONTRACT_TX_ID=YOUR_DEPLOYED_TX_ID
ENV ORACLE_PRIV_KEY=YOUR_KEY
CMD ["node", "main.js"]
