import Joi from 'joi';

const schema = Joi.object().keys({
  func: Joi.string().required(),
  type_params: Joi.string().optional(),
  params: Joi.string().max(500).required(),
});
