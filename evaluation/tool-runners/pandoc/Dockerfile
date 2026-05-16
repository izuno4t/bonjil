FROM pandoc/core:latest

COPY run.sh /usr/local/bin/bonjil-eval-pandoc
RUN chmod +x /usr/local/bin/bonjil-eval-pandoc

ENTRYPOINT ["/usr/local/bin/bonjil-eval-pandoc"]
