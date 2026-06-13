"""User data models."""


class UserProfile:
    def __init__(self, row):
        self.row = row or {}

    def to_dict(self):
        return {
            "id": self.row.get("id"),
            "email": self.row.get("email"),
            "name": self.row.get("name", ""),
        }
