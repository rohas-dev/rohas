import { CreateUserRequest, CreateUserResponse } from '@generated/api/create_user';
import { State } from '@generated/state';

export async function handleCreateUser(req: CreateUserRequest, state: State): Promise<CreateUserResponse> {
  // TODO: Implement handler logic
  // For auto-triggers (defined in schema triggers): use state.setPayload('EventName', {...})
  // For manual triggers: use state.triggerEvent('EventName', {...})
  throw new Error('Not implemented');
}
