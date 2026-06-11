import { m } from '$lib/paraglide/messages';
import { z } from 'zod';

export const editMemberSchema = z.object({
	id: z.number().int(m.id_must_integer()).positive(m.id_must_positive()),
	first_name: z.string().min(1, m.first_name_required()),
	last_name: z.string().min(1, m.last_name_required()),
	card_id: z
		.string()
		.regex(/^\d+$/, "Only numbers allowed")
		.transform(val => parseInt(val, 10))
		.refine(val => val >= 1 && val <= 1000, "Card number must be between 1 and 1000")
		.transform(val => val.toString()), // back to string
	email: z.string().email(m.invalid_email_format()).or(z.literal("")).optional().nullable(),
	phone: z.string().optional().nullable(),
	date_of_birth: z.string().optional().nullable()
});

export type EditMemberTypeSchema = typeof editMemberSchema;
