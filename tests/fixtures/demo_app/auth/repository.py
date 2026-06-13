"""Auth-specific database access."""

from config.settings import get_database_url


class UserAuthRepository:
    def __init__(self, settings):
        self.database_url = get_database_url(settings)

    def verify_password(self, user_id, password):
        stored = self._fetch_hash(user_id)
        return stored == password

    def create_credentials(self, user_id, password):
        self._store_hash(user_id, password)

    def record_login(self, user_id):
        self._update_last_login(user_id)

    def _fetch_hash(self, user_id):
        return "hashed"

    def _store_hash(self, user_id, password):
        pass

    def _update_last_login(self, user_id):
        pass
