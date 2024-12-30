use super::*;

impl SubscribeHeaderFlags {
    #[inline]
    pub fn parse<Input, Error>(input: &mut (Input, usize)) -> PResult<Self, Error>
    where
        Input: Stream<Token = u8> + StreamIsPartial + Clone,
        Error: ParserError<(Input, usize)> + AddContext<(Input, usize), StrContext>,
    {
        combinator::trace(
            type_name::<Self>(),
            bits::pattern(0b0000_0010, 4usize).value(Self),
        )
        .context(StrContext::Label(type_name::<Self>()))
        .context(StrContext::Expected(StrContextValue::Description(
            "SUBSCRIBE Header Flags",
        )))
        .parse_next(input)
    }
}

impl<'input> Subscribe<'input> {
    #[inline]
    pub fn parse<'settings, ByteInput, ByteError, BitError>(
        parser_settings: &'settings Settings,
    ) -> impl Parser<ByteInput, Self, ByteError> + use<'input, 'settings, ByteInput, ByteError, BitError>
    where
        ByteInput: StreamIsPartial + Stream<Token = u8, Slice = &'input [u8]> + Clone + UpdateSlice,
        ByteError: ParserError<ByteInput>
            + FromExternalError<ByteInput, Utf8Error>
            + FromExternalError<ByteInput, Utf8Error>
            + FromExternalError<ByteInput, InvalidQosError>
            + FromExternalError<ByteInput, InvalidPropertyTypeError>
            + FromExternalError<ByteInput, UnknownFormatIndicatorError>
            + AddContext<ByteInput, StrContext>,
        BitError: ParserError<(ByteInput, usize)>
            + ErrorConvert<ByteError>
            + FromExternalError<(ByteInput, usize), InvalidRetainHandlingError>
            + FromExternalError<(ByteInput, usize), InvalidQosError>
            + AddContext<(ByteInput, usize), StrContext>,
        BitError: ParserError<(ByteInput, usize)> + ErrorConvert<ByteError>,
    {
        combinator::trace(
            type_name::<Self>(),
            (
                combinator::trace("Packet ID", two_byte_integer.verify_map(NonZero::new)),
                SubscribeProperties::parse(parser_settings),
                combinator::trace(
                    "subscriptions",
                    combinator::repeat_till(
                        1..=parser_settings.max_subscriptions_len as usize,
                        Subscription::parse::<_, _, BitError>(parser_settings),
                        combinator::eof,
                    ),
                ),
            )
                .map(
                    move |(packet_id, properties, (subscriptions, _))| Subscribe {
                        packet_id,
                        subscriptions,
                        properties,
                    },
                ),
        )
    }
}

impl<'input> SubscribeProperties<'input> {
    #[inline]
    pub fn parse<'settings, Input, Error>(
        parser_settings: &'settings Settings,
    ) -> impl Parser<Input, Self, Error> + use<'input, 'settings, Input, Error>
    where
        Input: Stream<Token = u8, Slice = &'input [u8]> + UpdateSlice + StreamIsPartial + Clone,
        Error: ParserError<Input>
            + AddContext<Input, StrContext>
            + FromExternalError<Input, Utf8Error>
            + FromExternalError<Input, InvalidQosError>
            + FromExternalError<Input, InvalidPropertyTypeError>
            + FromExternalError<Input, UnknownFormatIndicatorError>,
    {
        combinator::trace(type_name::<Self>(), |input: &mut Input| {
            // TODO: Can't use binary::length_and_then because it doesn't work
            let data = binary::length_take(variable_byte_integer).parse_next(input)?;
            let mut input = input.clone().update_slice(data);
            let input = &mut input;

            let mut properties = Self::default();

            let mut parser = combinator::alt((
                combinator::eof.value(None),
                Property::parse(parser_settings).map(Some),
            ));

            while let Some(p) = parser.parse_next(input)? {
                match p {
                    Property::SubscriptionIdentifier(value) => {
                        properties.subscription_identifier.replace(value);
                    }
                    Property::UserProperty(key, value) => {
                        properties.user_properties.push((key, value))
                    }
                    _ => return Err(ErrMode::Cut(Error::assert(input, "Invalid property type"))),
                }
            }

            Ok(properties)
        })
        .context(StrContext::Label(type_name::<Self>()))
    }
}
