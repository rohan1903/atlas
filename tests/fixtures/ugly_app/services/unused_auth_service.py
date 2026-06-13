"""Never imported anywhere — ghost auth implementation."""


class UnusedAuthService:
    def login(self, email, password):
        return {"unused": True}

    def refresh(self, token):
        return {"unused_refresh": token}
