# Production's watch on the chain RPC and the nodes' reads (monitor.py); its
# alerts go to the owner's Telegram. Python's standard library only.
FROM python:3.13-alpine
COPY deploy/monitor/monitor.py /usr/local/bin/monitor.py
ENTRYPOINT ["python3", "/usr/local/bin/monitor.py"]
