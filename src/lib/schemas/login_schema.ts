import { z } from 'zod';

export const formSchema = z.object({
	username: z.string().min(2).max(20),
	password: z.string().min(5).max(20)
});

export type LoginSchema = typeof formSchema;
