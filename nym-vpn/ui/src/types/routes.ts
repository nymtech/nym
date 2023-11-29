import { routes } from '../constants.ts';

export type Routes = (typeof routes)[keyof typeof routes];
