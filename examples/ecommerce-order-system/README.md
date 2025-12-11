# E-commerce Order Processing System

A comprehensive, production-ready example demonstrating advanced event-driven architecture patterns with Rohas.

## Overview

This example showcases a complete e-commerce order processing system with:

- **Multi-step workflows**: Order creation → Payment → Inventory → Shipping → Delivery
- **Event-driven architecture**: Decoupled services communicating via events
- **Real-time updates**: WebSocket connections for live order status
- **Scheduled jobs**: Order expiration and inventory synchronization
- **Telemetry**: Full tracing and monitoring of the order lifecycle
- **Error handling**: Payment failures, inventory issues, order cancellations

## Features Demonstrated

### APIs
- `POST /orders` - Create a new order
- `GET /orders/:orderId` - Get order status
- `POST /orders/:orderId/cancel` - Cancel an order
- `GET /orders` - List all orders

### Events
- **Order Events**: OrderCreated, OrderCancelled, OrderStatusUpdated, OrderExpired, OrderShipped, OrderDelivered
- **Payment Events**: PaymentProcessed, PaymentCompleted, PaymentFailed, PaymentRefunded
- **Inventory Events**: InventoryReserved, InventoryReleased, InventoryLowStock, InventoryOutOfStock

### WebSockets
- `/ws/orders` - Real-time order status updates

### Cron Jobs
- **ExpirePendingOrders**: Runs every 5 minutes to expire unpaid orders
- **SyncInventory**: Runs every 6 hours to check inventory levels

### Middleware
- Authentication middleware
- Rate limiting middleware
- Request logging middleware

## Architecture

```
Order Creation Flow:
1. POST /orders → CreateOrder API
2. Triggers OrderCreated event
3. Parallel processing:
   - ProcessPayment event handler
   - ReserveInventory event handler
4. PaymentCompleted → CreateShipment
5. OrderShipped → Send notifications
6. OrderDelivered → Complete order
```

## Getting Started

### Prerequisites

- Rust 1.70+ (for Rohas CLI)
- Python 3.9+ (for handlers)
- Rohas CLI installed

### Installation

1. **Generate code from schemas:**
   ```bash
   cd ecommerce-order-system
   rohas codegen
   ```

2. **Start the development server:**
   ```bash
   rohas dev --workbench
   ```

   Or without workbench:
   ```bash
   rohas dev
   ```

3. **Validate schemas:**
   ```bash
   rohas validate
   ```

## Project Structure

```
ecommerce-order-system/
├── config/
│   └── rohas.toml          # Configuration
├── schema/
│   ├── models/            # Data models
│   │   ├── order.ro
│   │   ├── product.ro
│   │   ├── customer.ro
│   │   └── payment.ro
│   ├── api/               # API endpoints
│   │   └── order_api.ro
│   ├── events/            # Event definitions
│   │   ├── order_events.ro
│   │   ├── payment_events.ro
│   │   └── inventory_events.ro
│   ├── cron/              # Scheduled jobs
│   │   ├── order_expiration.ro
│   │   └── inventory_sync.ro
│   └── websockets/        # WebSocket endpoints
│       └── order_updates.ro
└── src/
    ├── handlers/
    │   ├── api/           # API handlers
    │   ├── events/        # Event handlers
    │   ├── cron/          # Cron handlers
    │   └── websockets/    # WebSocket handlers
    └── middlewares/       # Middleware functions
```

## Usage Examples

### Create an Order

```bash
curl -X POST http://localhost:4401/orders \
  -H "Content-Type: application/json" \
  -d '{
    "customerId": 1,
    "items": [
      {"productId": 1, "quantity": 2},
      {"productId": 2, "quantity": 1}
    ],
    "shippingAddress": "123 Main St, City, Country"
  }'
```

### Get Order Status

```bash
curl http://localhost:4401/orders/1
```

### Cancel an Order

```bash
curl -X POST http://localhost:4401/orders/1/cancel \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Customer requested cancellation"
  }'
```

### WebSocket Connection

Connect to `ws://localhost:4401/ws/orders` and send:

```json
{
  "type": "subscribe",
  "orderId": 1
}
```

## Event Flow

### Order Creation Flow

1. **CreateOrder API** → Creates order, triggers `OrderCreated`
2. **OrderCreated event** triggers:
   - `process_payment` handler → Processes payment
   - `reserve_inventory` handler → Reserves inventory
3. **PaymentProcessed** → Updates order payment status
4. **PaymentCompleted** → Creates shipment
5. **OrderShipped** → Sends shipping notification
6. **OrderDelivered** → Completes order

### Order Cancellation Flow

1. **CancelOrder API** → Cancels order, triggers `OrderCancelled`
2. **OrderCancelled event** triggers:
   - `release_inventory` handler → Releases reserved inventory
   - `refund_payment` handler → Processes refund
   - `send_cancellation_notification` handler → Notifies customer

## Telemetry

The system includes comprehensive telemetry:

- **Traces**: Full order lifecycle tracing
- **Metrics**: Order processing times, success rates
- **Logs**: Structured logging at each step

View telemetry in the Rohas Workbench UI when running with `--workbench` flag.

## Best Practices Demonstrated

1. **Event-Driven Architecture**: Decoupled services via events
2. **Saga Pattern**: Distributed transaction coordination
3. **Error Handling**: Graceful failure handling
4. **Idempotency**: Safe retries for event handlers
5. **Observability**: Full tracing and logging
6. **Real-time Updates**: WebSocket for live status

## Next Steps

- Add database persistence (currently uses in-memory storage)
- Implement actual payment gateway integration
- Add inventory management system
- Implement user authentication
- Add more sophisticated error handling and retries

## License

MIT License - see LICENSE file for details.
