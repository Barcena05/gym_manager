import { m } from '$lib/paraglide/messages';
import { z } from 'zod';

export const newMemberSchema = z.object({
	first_name: z.string().min(1, m.first_name_required()),
	last_name: z.string().min(1, m.last_name_required()),
	card_id: z
		.string()
		.trim()
		.min(3, 'El número de tarjeta debe tener al menos 3 caracteres')
		.max(64, 'El número de tarjeta no puede tener más de 64 caracteres')
		.regex(/^[A-Za-z0-9\-_]+$/, 'Solo se permiten letras, números, guiones y guiones bajos'),
	email: z.string().email(m.invalid_email_format()).or(z.literal('')).optional().nullable(),
	phone: z.string().optional().nullable(),
	date_of_birth: z.string().optional().nullable()
});

export type NewMemberTypeSchema = typeof newMemberSchema;
