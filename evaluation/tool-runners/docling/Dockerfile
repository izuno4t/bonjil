FROM python:3.12-slim

RUN pip install --no-cache-dir docling

COPY run.py /opt/bonjil-eval/run.py

ENTRYPOINT ["python", "/opt/bonjil-eval/run.py"]
