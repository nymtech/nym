export type PrivacyLevel = 'High' | 'Medium';

export type SelectedGateway = {
  address?: string;
  is_active?: boolean;
};

export type SelectedSp = {
  address?: string;
  is_active?: boolean;
};

export type UserData = {
  monitoring?: boolean;
  privacy_level?: PrivacyLevel;
  selected_gateway?: SelectedGateway;
  selected_sp?: SelectedSp;
};
