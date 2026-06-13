"""Order persistence."""

from config.settings import get_database_url
from utils.logger import log_info


class OrderRepository:
    def __init__(self, settings):
        self.database_url = get_database_url(settings)

    def create_order(self, user_id, items):
        log_info("OrderRepository.create_order")
        return {"id": 100, "user_id": user_id, "items": items, "status": "pending"}

    def get_by_id(self, order_id):
        return {"id": order_id, "user_id": 1, "status": "pending", "items": []}

    def list_for_user(self, user_id):
        return [{"id": 100, "user_id": user_id, "status": "pending"}]

    def update_status(self, order_id, status):
        log_info(f"order {order_id} -> {status}")
        return {"id": order_id, "status": status}
