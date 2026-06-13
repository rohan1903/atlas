"""User profile business logic."""

from users.repository import UserRepository
from users.models import UserProfile
from utils.logger import log_info


class UserService:
    def __init__(self, settings):
        self.repo = UserRepository(settings)

    def list_users(self):
        log_info("UserService.list_users")
        rows = self.repo.fetch_all()
        return [UserProfile(row).to_dict() for row in rows]

    def get_user(self, user_id):
        row = self.repo.get_by_id(user_id)
        return UserProfile(row).to_dict()

    def update_profile(self, user_id, data):
        self.repo.update_user(user_id, data)
        return self.get_user(user_id)
