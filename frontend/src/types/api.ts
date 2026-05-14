export interface LoginPayload {
  email: string;
  password: string;
}

export interface RegisterPayload {
  email: string;
  username: string;
  password: string;
}

export interface LoginRes {
  user_id: string;
  email: string;
  access_token: string;
}

export interface User {
  id: string;
  email: string;
}
