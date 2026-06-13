"""Legacy auth service — circular import with legacy_final_fixed."""

from legacy_final_fixed.auth_service_final_v2_fixed import AuthServiceFinalV2Fixed


class AuthService:
    def login(self, email, password):
        fixed = AuthServiceFinalV2Fixed({})
        return fixed.login(email, password)

    def authenticate(self, token):
        return token is not None
