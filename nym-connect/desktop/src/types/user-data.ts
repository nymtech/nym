export type PrivacyMode = 'High' | 'Medium';

export type UserData = {
  monitoring?: boolean;
  privacy_mode?: PrivacyMode;
};
