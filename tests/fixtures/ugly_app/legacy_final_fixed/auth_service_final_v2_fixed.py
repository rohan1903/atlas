"""Circular import partner — imports legacy which imports this module."""

from legacy.auth_service import AuthService


class AuthServiceFinalV2Fixed:
    def login(self, email, password):
        bridge = AuthService()
        return bridge.login(email, password)
