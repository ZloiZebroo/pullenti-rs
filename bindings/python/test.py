from pullentipy import Sdk, PersonAnalyzer, GeoAnalyzer, OrgAnalyzer

# Init SDK 
Sdk.initialize_with(
    lang='ru',
    analyzers=[PersonAnalyzer(), GeoAnalyzer(), OrgAnalyzer()],
)

# init processor
proc = Sdk.create_processor()

# sample text
text = 'Евгений Петрович живёт в Санкт-Петербурге и работает в ООО Ромашка.'

# run
result = proc.analyze(text)

# result
print("\n=== done ===")
print(result)
for ent in result.referents:
    print(f"  {ent.entity_type}: {ent.text!r}")
    for s in ent.slots:
        print(f"    {s.name} = {s.value}")
