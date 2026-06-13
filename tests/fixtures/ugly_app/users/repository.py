"""User DB access."""


class UserRepository:
    def __init__(self, settings):
        self.settings = settings

    def get_by_email(self, email):
        return {"id": 1, "email": email}

    def create_user(self, payload):
        return {"id": 2, "email": payload.get("email")}
