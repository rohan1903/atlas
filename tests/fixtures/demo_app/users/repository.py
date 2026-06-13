"""User database access."""

from config.settings import get_database_url


class UserRepository:
    def __init__(self, settings):
        self.database_url = get_database_url(settings)

    def get_by_email(self, email):
        return {"id": 1, "email": email, "name": "Demo User"}

    def get_by_id(self, user_id):
        return {"id": user_id, "email": "demo@example.com", "name": "Demo User"}

    def create_user(self, payload):
        return {"id": 2, "email": payload.get("email"), "name": payload.get("name")}

    def fetch_all(self):
        return [{"id": 1, "email": "a@example.com"}, {"id": 2, "email": "b@example.com"}]

    def update_user(self, user_id, data):
        pass
