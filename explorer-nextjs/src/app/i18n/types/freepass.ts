export interface RedeemFreePassCopy {
  errors: {
    invalidCode: string;
    alreadyRedeemed: string;
    unknown: string;
  };
}

type GetStarted = {
  title: string;
  description: string;
  ctaText: string;
  inputLabel: string;
  prompt: string;
};

export type AlphaPageSteps = {
  title: string;
  description: string;
};

export type AlphaPageTranslations = {
  hero: {
    title: {
      firstLine: string;
      secondLine?: string;
    };
    getCredential: GetStarted;
    signUp: GetStarted;
    imageAlt: string;
    stepsTitle: string;
    finalSteps: AlphaPageSteps[];
    finalStepsAlert: string;
    credentialsTitle: string;
    credentialsButton: string;
  };
  redeemFreePass: RedeemFreePassCopy;
  firstSection: {
    title: string;
    content: string[];
  };
  secondSection: {
    title: string;
    content: string[];
  };
};
