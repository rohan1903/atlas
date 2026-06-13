"""Email notifications (leaf utility)."""

from utils.logger import log_info


def send_welcome_email(user):
    log_info(f"welcome email to {user.get('email')}")
    return {"sent": True}


def send_order_confirmation(user, order):
    log_info(f"order confirmation for order {order.get('id')}")
    return {"sent": True}
