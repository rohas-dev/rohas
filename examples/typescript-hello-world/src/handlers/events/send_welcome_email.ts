import { UserCreated } from '@generated/events/user_created';

export async function send_welcome_email(event: UserCreated): Promise<void> {
  // TODO: Implement event handler
  console.log('Handling event:', event);
}
