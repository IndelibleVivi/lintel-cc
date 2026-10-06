import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { homeGreeting } from './homeGreetings';
import './styles.css';
import './fonts';

let greetingStorage: Storage | undefined;
try { greetingStorage = window.localStorage; } catch { /* Optional persistence. */ }
// Outside render: StrictMode must not consume the once-per-date greeting twice.
const greeting = homeGreeting(new Date(), Math.random, greetingStorage);
createRoot(document.getElementById('root')!).render(<React.StrictMode><App greeting={greeting} /></React.StrictMode>);
