FROM node:22-slim

WORKDIR /opt/bonjil-eval
RUN npm install mammoth turndown

COPY run.mjs /opt/bonjil-eval/run.mjs

ENTRYPOINT ["node", "/opt/bonjil-eval/run.mjs"]
