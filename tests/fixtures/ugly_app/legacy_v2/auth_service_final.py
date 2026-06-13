"""Duplicate AuthService — v2 naming convention."""


class AuthServiceFinal:
    def login(self, email, password):
        return {"legacy_v2": True, "email": email}
