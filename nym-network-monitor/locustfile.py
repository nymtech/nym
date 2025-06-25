import time
import logging
from locust import HttpUser, task
from requests.exceptions import ConnectionError

# Configure logging to see what's happening
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class SendMsg(HttpUser):
    @task
    def hello_world(self):
        try:
            response = self.client.post("/v1/send")
            if response.status_code == 503:
                logger.warning(f"Got 503 Service Unavailable, sleeping for 1 second")
                time.sleep(1)
            response.raise_for_status()  # Raise an exception for bad status codes (4xx or 5xx)
        except ConnectionError as e:
            # This catches ConnectionRefused errors
            logger.error(f"Connection refused, backing off for 5 seconds: {e}")
            time.sleep(5)  # Longer pause for connection errors
        except Exception as e:
            # Log other errors but don't sleep as long
            logger.warning(f"Request failed: {type(e).__name__}: {e}")
            time.sleep(0.5)  # Brief pause for other errors
