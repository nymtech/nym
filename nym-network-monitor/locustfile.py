from locust import HttpUser, task

class SendMsg(HttpUser):
    @task
    def hello_world(self):
        self.client.get("/")
