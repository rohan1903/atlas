"""Canonical token helpers."""

import time


class TokenService:
    def __init__(self, settings):
        self.settings = settings

    def create_access_token(self, user_id):
        return f"token-{user_id}-{int(time.time())}"

    def verify_access_token(self, token):
        return token.startswith("token-")

    def revoke_user_tokens(self, user_id):
        return user_id
