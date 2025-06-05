import time
from locust import HttpUser, task


class SendMsg(HttpUser):
    @task
    def hello_world(self):
        try:
            response = self.client.post("/v1/send", timeout=10)
            if response.status_code == 503:
                time.sleep(1)
            response.raise_for_status()  # Raise an exception for bad status codes (4xx or 5xx)
        except Exception: # Catch other exceptions, including those raised by raise_for_status()
            # You might want to log this error or handle it differently
            pass
