# Frontend dev server for the local Docker stack (see ../docker-compose.yml).
# Production builds the static site instead; see ../DEPLOYMENT.md.
FROM node:20-slim
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
EXPOSE 5173
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
