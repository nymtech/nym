export interface ServiceProvider {
  id: string;
  description: string;
  address: string;
  gateway: string;
}

export interface Service {
  id: string;
  description: string;
  items: ServiceProvider[];
}

export type Services = Service[];
