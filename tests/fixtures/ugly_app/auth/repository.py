"""Canonical credential storage."""


class UserAuthRepository:
    def __init__(self, settings):
        self.settings = settings

    def verify_password(self, user_id, password):
        stored = self._fetch_hash(user_id)
        return stored == password

    def create_credentials(self, user_id, password):
        return {"user_id": user_id}

    def record_login(self, user_id):
        return user_id

    def _fetch_hash(self, user_id):
        return f"hash-{user_id}"
