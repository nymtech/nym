import { routes } from '../constants';

export type Routes = (typeof routes)[keyof typeof routes];
