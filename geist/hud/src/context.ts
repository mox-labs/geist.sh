import { createContext } from '@lit/context';
import type { GeistController } from './controller.js';

/** Lit context key for sharing GeistController across child elements. */
export const geistContext = createContext<GeistController>('geist');
