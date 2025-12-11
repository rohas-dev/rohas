import asyncio
from generated.state import State
from generated.events.inventory_out_of_stock import InventoryOutOfStock


async def handle_send_out_of_stock_alert(event: InventoryOutOfStock, state: State) -> None:
    """Send alert when product is out of stock."""
    product_id = event.payload.get('productId')

    state.logger.error(f'Out of stock alert: Product {product_id} is out of stock')

    await asyncio.sleep(0.1)
    # In a real app, send urgent notification
