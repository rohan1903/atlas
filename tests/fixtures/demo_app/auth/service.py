"""Authentication business logic."""

from auth.repository import UserAuthRepository
from auth.token import TokenService
from users.repository import UserRepository
from utils.logger import log_info, log_error


class AuthService:
    def __init__(self, settings):
        self.settings = settings
        self.auth_repo = UserAuthRepository(settings)
        self.user_repo = UserRepository(settings)
        self.tokens = TokenService(settings)

    def login(self, email, password):
        log_info("AuthService.login")
        user = self.user_repo.get_by_email(email)
        if not user:
            log_error("user not found")
            return None
        if not self.auth_repo.verify_password(user["id"], password):
            log_error("invalid password")
            return None
        token = self.tokens.create_access_token(user["id"])
        self.auth_repo.record_login(user["id"])
        return {"token": token, "user_id": user["id"]}

    def register(self, payload):
        log_info("AuthService.register")
        user = self.user_repo.create_user(payload)
        self.auth_repo.create_credentials(user["id"], payload.get("password"))
        token = self.tokens.create_access_token(user["id"])
        return {"token": token, "user_id": user["id"]}

    def logout(self, user_id):
        self.tokens.revoke_user_tokens(user_id)
        return {"ok": True}

    def authenticate(self, token):
        return self.tokens.verify_access_token(token)
