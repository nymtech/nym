export interface ServiceProvider {
  id: string;
  description: string;
  address: string;
}

export interface Service {
  id: string;
  description: string;
  items: ServiceProvider[];
}

export type Services = Service[];

export interface Gateway {
  identity: string;
}
