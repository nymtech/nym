export type PrivacyLevel = 'High' | 'Medium';

export type UserData = {
  monitoring?: boolean;
  privacy_level?: PrivacyLevel;
};
